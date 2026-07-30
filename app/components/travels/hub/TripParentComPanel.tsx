"use client";

import { useEffect, useMemo, useState } from "react";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { parentEmailCoverage } from "@/app/lib/travels-eleves-list";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { TripAlert, TripButton, TripInput, TripSection, TripTextarea } from "@/app/components/travels/TripDetailUI";

type PhotoDraft = {
  id: string;
  filename: string;
  contentType: string;
  contentBase64: string;
  previewUrl: string;
};

type Props = {
  trip: TravelsTrip;
  canEdit: boolean;
  onTripUpdated: (trip: TravelsTrip) => void;
};

const MAX_PHOTOS = 12;

async function compressImageFile(file: File): Promise<PhotoDraft> {
  const bitmap = await createImageBitmap(file);
  const maxSide = 1600;
  const scale = Math.min(1, maxSide / Math.max(bitmap.width, bitmap.height));
  const w = Math.round(bitmap.width * scale);
  const h = Math.round(bitmap.height * scale);
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas indisponible");
  ctx.drawImage(bitmap, 0, 0, w, h);
  bitmap.close();

  const blob: Blob = await new Promise((resolve, reject) => {
    canvas.toBlob(
      (b) => (b ? resolve(b) : reject(new Error("Compression échouée"))),
      "image/jpeg",
      0.72,
    );
  });
  if (blob.size > 900_000) {
    const tighter: Blob = await new Promise((resolve, reject) => {
      canvas.toBlob(
        (b) => (b ? resolve(b) : reject(new Error("Compression échouée"))),
        "image/jpeg",
        0.55,
      );
    });
    if (tighter.size > 900_000) throw new Error(`${file.name} reste trop volumineuse après compression.`);
    const buf = await tighter.arrayBuffer();
    return {
      id: `${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      filename: file.name.replace(/\.\w+$/, ".jpg"),
      contentType: "image/jpeg",
      contentBase64: bufferToBase64(buf),
      previewUrl: URL.createObjectURL(tighter),
    };
  }
  const buf = await blob.arrayBuffer();
  return {
    id: `${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
    filename: file.name.replace(/\.\w+$/, ".jpg"),
    contentType: "image/jpeg",
    contentBase64: bufferToBase64(buf),
    previewUrl: URL.createObjectURL(blob),
  };
}

function bufferToBase64(buf: ArrayBuffer) {
  const bytes = new Uint8Array(buf);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

export function TripParentComPanel({ trip, canEdit, onTripUpdated }: Props) {
  const [eleves, setEleves] = useState<EleveConfig[]>([]);
  const [subject, setSubject] = useState(`Sortie — ${trip.data.title || trip.data.destination || ""}`);
  const [message, setMessage] = useState("");
  const [photos, setPhotos] = useState<PhotoDraft[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    fetch("/api/eleves")
      .then((r) => r.json())
      .then((j) => setEleves(Array.isArray(j.eleves) ? j.eleves : []))
      .catch(() => setEleves([]));
  }, []);

  useEffect(() => {
    return () => {
      for (const p of photos) URL.revokeObjectURL(p.previewUrl);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const participants = trip.data.participantEleves || [];
  const elevesByIne = useMemo(() => new Map(eleves.map((e) => [e.ine, e])), [eleves]);
  const coverage = useMemo(
    () => parentEmailCoverage(participants, elevesByIne),
    [participants, elevesByIne],
  );
  const logs = trip.data.parentComLogs || [];

  const onPickFiles = async (files: FileList | null) => {
    if (!files?.length) return;
    const remaining = MAX_PHOTOS - photos.length;
    if (remaining <= 0) return alert(`Maximum ${MAX_PHOTOS} photos.`);
    const slice = [...files].slice(0, remaining);
    try {
      const next: PhotoDraft[] = [];
      for (const f of slice) {
        if (!f.type.startsWith("image/")) continue;
        next.push(await compressImageFile(f));
      }
      setPhotos((prev) => [...prev, ...next]);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Compression impossible");
    }
  };

  const removePhoto = (id: string) => {
    setPhotos((prev) => {
      const target = prev.find((p) => p.id === id);
      if (target) URL.revokeObjectURL(target.previewUrl);
      return prev.filter((p) => p.id !== id);
    });
  };

  const send = async () => {
    if (!canEdit) return;
    if (!subject.trim() || !message.trim()) return alert("Sujet et message requis.");
    if (coverage.emails.length === 0) {
      return alert("Aucun e-mail parent pour les élèves de la liste.");
    }
    if (
      !confirm(
        `Envoyer à ${coverage.emails.length} destinataire(s) parent${coverage.emails.length > 1 ? "s" : ""} ?`,
      )
    ) {
      return;
    }
    setBusy(true);
    try {
      const res = await fetch("/api/travels/send-parents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: trip.id,
          subject: subject.trim(),
          body: message.trim(),
          photos: photos.map((p) => ({
            filename: p.filename,
            contentType: p.contentType,
            contentBase64: p.contentBase64,
          })),
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Envoi impossible");
      onTripUpdated(j.trip as TravelsTrip);
      for (const p of photos) URL.revokeObjectURL(p.previewUrl);
      setPhotos([]);
      setMessage("");
      alert(`Message envoyé à ${j.recipientCount} destinataire(s).`);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  if (participants.length === 0) {
    return (
      <TripSection title="Communication parents" icon="📸" accent="amber">
        <div className="px-6 py-5">
          <TripAlert tone="info" title="Liste requise">
            Composez d’abord la liste des élèves (onglet Élèves) pour communiquer aux familles.
          </TripAlert>
        </div>
      </TripSection>
    );
  }

  return (
    <TripSection
      title="Communication parents"
      subtitle="Sens unique établissement → familles · plusieurs envois possibles"
      icon="📸"
      accent="amber"
    >
      <div className="px-6 py-5 space-y-5">
        <TripAlert tone="info" icon="✉️" title="Canal mail">
          Les photos ne sont pas stockées durablement dans ScolIA — elles sont jointes à l’e-mail
          uniquement. Pas de messagerie inverse : les parents ne répondent pas dans la plateforme.
        </TripAlert>

        <p className="text-sm text-slate-600">
          Destinataires : <strong>{coverage.emails.length}</strong> e-mail
          {coverage.emails.length > 1 ? "s" : ""} parent
          {coverage.withoutMail > 0
            ? ` · ${coverage.withoutMail} élève(s) sans mail (non couverts)`
            : ""}
          .
        </p>

        <div className="space-y-3">
          <div>
            <label className="block text-xs font-semibold text-slate-500 uppercase mb-1">Sujet</label>
            <TripInput
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              disabled={!canEdit || busy}
            />
          </div>
          <div>
            <label className="block text-xs font-semibold text-slate-500 uppercase mb-1">Message</label>
            <TripTextarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={5}
              disabled={!canEdit || busy}
              placeholder="Ex. Bonjour, voici quelques photos de la journée…"
            />
          </div>
          <div>
            <label className="block text-xs font-semibold text-slate-500 uppercase mb-2">
              Photos ({photos.length}/{MAX_PHOTOS})
            </label>
            {canEdit && (
              <input
                type="file"
                accept="image/*"
                multiple
                disabled={busy || photos.length >= MAX_PHOTOS}
                onChange={(e) => {
                  void onPickFiles(e.target.files);
                  e.target.value = "";
                }}
                className="text-sm"
              />
            )}
            {photos.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-2">
                {photos.map((p) => (
                  <div key={p.id} className="relative w-20 h-20 rounded-lg overflow-hidden ring-1 ring-slate-200">
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img src={p.previewUrl} alt="" className="w-full h-full object-cover" />
                    {canEdit && (
                      <button
                        type="button"
                        onClick={() => removePhoto(p.id)}
                        className="absolute top-0.5 right-0.5 bg-black/60 text-white text-[10px] rounded px-1"
                      >
                        ×
                      </button>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {canEdit && (
          <TripButton variant="primary" disabled={busy} onClick={send}>
            {busy ? "Envoi…" : "Envoyer aux parents"}
          </TripButton>
        )}

        {logs.length > 0 && (
          <div className="pt-4 border-t border-slate-100">
            <h3 className="text-sm font-semibold text-slate-700 mb-2">Historique des envois</h3>
            <ul className="space-y-2 text-sm text-slate-600">
              {[...logs].reverse().map((l) => (
                <li key={l.id} className="rounded-lg bg-slate-50 px-3 py-2">
                  <div className="font-medium text-slate-800">{l.subject}</div>
                  <div className="text-xs text-slate-500 mt-0.5">
                    {new Date(l.sentAt).toLocaleString("fr-FR")} · {l.recipientCount} destinataire
                    {l.recipientCount > 1 ? "s" : ""} · {l.photoCount} photo
                    {l.photoCount > 1 ? "s" : ""}
                    {l.sentBy.name ? ` · ${l.sentBy.name}` : ""}
                  </div>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </TripSection>
  );
}
