"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { parentEmailCoverage } from "@/app/lib/travels-eleves-list";
import {
  defaultParentCalendarFromTrip,
  newCalendarPointId,
} from "@/app/lib/travels-parent-calendar";
import type {
  TravelsCalendarPoint,
  TravelsParentBlogDelegate,
  TravelsParentBlogMeta,
  TravelsParentCalendar,
  TravelsTrip,
} from "@/app/lib/travels-types";
import { TripAlert, TripButton, TripInput, TripSection, TripTextarea } from "@/app/components/travels/TripDetailUI";

type PhotoDraft = {
  id: string;
  filename: string;
  contentType: string;
  contentBase64: string;
  previewUrl: string;
};

type BlogPostView = {
  id: string;
  authorName: string;
  body: string;
  createdAt: string;
  photos: Array<{ id: string; url: string; contentType?: string }>;
};

type DirectoryUser = {
  externalUserId: string;
  email: string;
  displayName: string;
};

type Props = {
  trip: TravelsTrip;
  canEdit: boolean;
  onTripUpdated: (trip: TravelsTrip) => void;
};

const MAX_PHOTOS = 12;

const KIND_OPTIONS: { value: TravelsCalendarPoint["kind"]; label: string }[] = [
  { value: "depot", label: "Dépôt / départ" },
  { value: "recuperation", label: "Récupération / retour" },
  { value: "autre", label: "Autre point" },
];

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
  const [message, setMessage] = useState("");
  const [photos, setPhotos] = useState<PhotoDraft[]>([]);
  const [busy, setBusy] = useState(false);
  const [calendar, setCalendar] = useState<TravelsParentCalendar>(() =>
    defaultParentCalendarFromTrip(trip.data),
  );
  const [blog, setBlog] = useState<TravelsParentBlogMeta | null>(trip.data.parentBlog || null);
  const [expired, setExpired] = useState(false);
  const [canPublish, setCanPublish] = useState(false);
  const [posts, setPosts] = useState<BlogPostView[]>([]);
  const [delegates, setDelegates] = useState<TravelsParentBlogDelegate[]>(
    trip.data.parentBlogDelegates || [],
  );
  const [directory, setDirectory] = useState<DirectoryUser[]>([]);
  const [delegatePick, setDelegatePick] = useState("");

  useEffect(() => {
    setCalendar(defaultParentCalendarFromTrip(trip.data));
  }, [trip.id, trip.data.parentCalendar, trip.data.parentMeeting, trip.data.startDate, trip.data.endDate]);

  useEffect(() => {
    setBlog(trip.data.parentBlog || null);
    setDelegates(trip.data.parentBlogDelegates || []);
  }, [trip.id, trip.data.parentBlog, trip.data.parentBlogDelegates]);

  useEffect(() => {
    fetch("/api/eleves")
      .then((r) => r.json())
      .then((j) => setEleves(Array.isArray(j.eleves) ? j.eleves : []))
      .catch(() => setEleves([]));
  }, []);

  useEffect(() => {
    fetch("/api/travels/directory-escorts", { cache: "no-store" })
      .then((r) => r.json())
      .then((j) => setDirectory(Array.isArray(j.users) ? j.users : []))
      .catch(() => setDirectory([]));
  }, []);

  const reloadBlog = useCallback(async () => {
    try {
      const res = await fetch(`/api/travels/parent-blog?tripId=${encodeURIComponent(trip.id)}`, {
        cache: "no-store",
      });
      const j = await res.json();
      if (!res.ok) return;
      setBlog(j.blog || null);
      setExpired(Boolean(j.expired));
      setCanPublish(Boolean(j.canEdit));
      setPosts(Array.isArray(j.posts) ? j.posts : []);
      if (Array.isArray(j.delegates)) setDelegates(j.delegates);
    } catch {
      /* ignore */
    }
  }, [trip.id]);

  useEffect(() => {
    void reloadBlog();
  }, [reloadBlog]);

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

  const publicUrl = useMemo(() => {
    if (!blog?.publicPath) return "";
    if (typeof window === "undefined") return blog.publicPath;
    return `${window.location.origin}${blog.publicPath}`;
  }, [blog?.publicPath]);

  const updatePoint = (id: string, patch: Partial<TravelsCalendarPoint>) => {
    setCalendar((c) => ({
      ...c,
      points: c.points.map((p) => (p.id === id ? { ...p, ...patch } : p)),
    }));
  };

  const addPoint = (kind: TravelsCalendarPoint["kind"]) => {
    const startDate = String(trip.data.startDate || trip.data.date || "").slice(0, 10);
    setCalendar((c) => ({
      ...c,
      points: [
        ...c.points,
        {
          id: newCalendarPointId(),
          kind,
          label: KIND_OPTIONS.find((k) => k.value === kind)?.label,
          date: startDate || "",
          time: kind === "recuperation" ? "20:00" : "10:00",
          durationMinutes: 30,
          place: "",
        },
      ],
    }));
  };

  const removePoint = (id: string) => {
    setCalendar((c) => ({ ...c, points: c.points.filter((p) => p.id !== id) }));
  };

  const saveCalendar = async () => {
    if (!canEdit) return;
    setBusy(true);
    try {
      const updatedTrip: TravelsTrip = {
        ...trip,
        data: { ...trip.data, parentCalendar: calendar },
        history: [
          ...(trip.history || []),
          {
            date: new Date().toISOString(),
            user: "Équipe",
            action: "CALENDRIER_PARENTS_MAJ",
            note: `${calendar.points.length} point(s)`,
          },
        ],
      };
      const res = await fetch("/api/travels/update", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: trip.id, data: updatedTrip }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      onTripUpdated((j.trip || updatedTrip) as TravelsTrip);
      alert("Calendrier parents enregistré.");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const activateBlog = async () => {
    if (!canEdit) return;
    if (
      !confirm(
        "Activer la page de suivi parents ? Un e-mail unique partira aux familles (calendrier .ics + lien). Aucun mail ne sera envoyé à chaque publication.",
      )
    ) {
      return;
    }
    setBusy(true);
    try {
      const res = await fetch("/api/travels/parent-blog", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: trip.id,
          notifyParents: true,
          attachIcs: true,
          parentCalendar: calendar,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Activation impossible");
      if (j.trip) onTripUpdated(j.trip as TravelsTrip);
      setBlog(j.blog || null);
      await reloadBlog();
      const bits = [
        j.parentsNotified ? `Mail envoyé à ${j.parentsNotified} destinataire(s)` : null,
        j.parentsSkippedReason || null,
      ].filter(Boolean);
      alert(bits.length ? `Page activée.\n${bits.join("\n")}` : "Page activée.");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const copyLink = async () => {
    if (!publicUrl) return;
    try {
      await navigator.clipboard.writeText(publicUrl);
      alert("Lien copié.");
    } catch {
      prompt("Copiez le lien :", publicUrl);
    }
  };

  const saveDelegates = async (next: TravelsParentBlogDelegate[]) => {
    if (!canEdit) return;
    setBusy(true);
    try {
      const res = await fetch("/api/travels/parent-blog/delegates", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId: trip.id, delegates: next }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      if (j.trip) onTripUpdated(j.trip as TravelsTrip);
      setDelegates(j.delegates || next);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const addDelegate = () => {
    if (!delegatePick) return;
    const u = directory.find((d) => d.externalUserId === delegatePick);
    if (!u) return;
    if (delegates.some((d) => d.userId === u.externalUserId)) {
      return alert("Cette personne est déjà déléguée.");
    }
    const next = [
      ...delegates,
      { userId: u.externalUserId, name: u.displayName, email: u.email },
    ];
    setDelegatePick("");
    void saveDelegates(next);
  };

  const removeDelegate = (userId: string) => {
    void saveDelegates(delegates.filter((d) => d.userId !== userId));
  };

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

  const publish = async () => {
    if (!canEdit || !canPublish) return;
    if (!message.trim()) return alert("Message requis.");
    setBusy(true);
    try {
      const res = await fetch("/api/travels/parent-blog/posts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: trip.id,
          body: message.trim(),
          photos: photos.map((p) => ({
            filename: p.filename,
            contentType: p.contentType,
            contentBase64: p.contentBase64,
          })),
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Publication impossible");
      if (j.trip) onTripUpdated(j.trip as TravelsTrip);
      for (const p of photos) URL.revokeObjectURL(p.previewUrl);
      setPhotos([]);
      setMessage("");
      await reloadBlog();
      alert("Publication visible sur la page parents (pas de nouveau mail).");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const deletePost = async (postId: string) => {
    if (!canEdit) return;
    if (!confirm("Supprimer cette publication ?")) return;
    setBusy(true);
    try {
      const res = await fetch("/api/travels/parent-blog/posts", {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId: trip.id, postId }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Suppression impossible");
      if (j.trip) onTripUpdated(j.trip as TravelsTrip);
      await reloadBlog();
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
            Composez d’abord la liste des élèves (onglet Élèves) pour activer la page de suivi ou
            gérer le calendrier parents.
          </TripAlert>
        </div>
      </TripSection>
    );
  }

  return (
    <TripSection
      title="Communication parents"
      subtitle="Calendrier .ics · page de suivi publique (lecture seule) · publications équipe"
      icon="📸"
      accent="amber"
    >
      <div className="px-6 py-5 space-y-5">
        <TripAlert tone="info" icon="✉️" title="Comment ça marche">
          À la confirmation de liste (ou ici), vous pouvez activer une page publique pour les
          familles. Un seul mail part au départ (horaires + .ics + lien). Ensuite, les parents
          consultent librement — aucun mail à chaque publication. La page se ferme 15 jours après
          le retour.
        </TripAlert>

        <div className="rounded-xl border border-sky-200 bg-sky-50/60 p-4 space-y-3">
          <div>
            <h3 className="text-sm font-black text-sky-950">Calendrier parents (.ics)</h3>
            <p className="text-xs text-sky-900/80 mt-1">
              Points de dépôt / récupération joints au mail d’activation ou de confirmation.
            </p>
          </div>

          <label className="flex items-center gap-2 text-sm text-slate-800 cursor-pointer">
            <input
              type="checkbox"
              checked={calendar.includeTripSpan !== false}
              onChange={(e) =>
                setCalendar((c) => ({ ...c, includeTripSpan: e.target.checked }))
              }
              disabled={!canEdit || busy}
              className="h-4 w-4"
            />
            Inclure l’événement « séjour » (du départ au retour)
          </label>

          {calendar.points.map((pt, idx) => (
            <div
              key={pt.id}
              className="rounded-lg border border-white bg-white/90 p-3 space-y-2 shadow-sm"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="text-[11px] font-black uppercase tracking-wide text-slate-500">
                  Point {idx + 1}
                </span>
                {canEdit ? (
                  <button
                    type="button"
                    className="text-xs font-bold text-rose-600"
                    onClick={() => removePoint(pt.id)}
                  >
                    Supprimer
                  </button>
                ) : null}
              </div>
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
                <label className="block text-xs font-semibold text-slate-600">
                  Type
                  <select
                    className="mt-1 w-full rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-sm"
                    value={pt.kind}
                    disabled={!canEdit || busy}
                    onChange={(e) =>
                      updatePoint(pt.id, {
                        kind: e.target.value as TravelsCalendarPoint["kind"],
                      })
                    }
                  >
                    {KIND_OPTIONS.map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="block text-xs font-semibold text-slate-600">
                  Date
                  <TripInput
                    type="date"
                    className="mt-1"
                    value={pt.date}
                    onChange={(e) => updatePoint(pt.id, { date: e.target.value })}
                    disabled={!canEdit || busy}
                  />
                </label>
                <label className="block text-xs font-semibold text-slate-600">
                  Heure
                  <TripInput
                    type="time"
                    className="mt-1"
                    value={pt.time}
                    onChange={(e) => updatePoint(pt.id, { time: e.target.value })}
                    disabled={!canEdit || busy}
                  />
                </label>
                <label className="block text-xs font-semibold text-slate-600">
                  Lieu
                  <TripInput
                    className="mt-1"
                    value={pt.place || ""}
                    onChange={(e) => updatePoint(pt.id, { place: e.target.value })}
                    placeholder="Cour, parking…"
                    disabled={!canEdit || busy}
                  />
                </label>
              </div>
              <label className="block text-xs font-semibold text-slate-600">
                Note (optionnel)
                <TripInput
                  className="mt-1"
                  value={pt.note || ""}
                  onChange={(e) => updatePoint(pt.id, { note: e.target.value })}
                  placeholder="Ex. arriver 10 min avant"
                  disabled={!canEdit || busy}
                />
              </label>
            </div>
          ))}

          {canEdit ? (
            <div className="flex flex-wrap gap-2">
              <TripButton variant="secondary" disabled={busy} onClick={() => addPoint("depot")}>
                + Dépôt
              </TripButton>
              <TripButton
                variant="secondary"
                disabled={busy}
                onClick={() => addPoint("recuperation")}
              >
                + Récupération
              </TripButton>
              <TripButton variant="secondary" disabled={busy} onClick={() => addPoint("autre")}>
                + Autre point
              </TripButton>
              <TripButton variant="primary" disabled={busy} onClick={() => void saveCalendar()}>
                Enregistrer le calendrier
              </TripButton>
            </div>
          ) : null}
        </div>

        <div className="rounded-xl border border-violet-200 bg-violet-50/60 p-4 space-y-3">
          <div>
            <h3 className="text-sm font-black text-violet-950">Page de suivi parents</h3>
            <p className="text-xs text-violet-900/80 mt-1">
              Destinataires potentiels : <strong>{coverage.emails.length}</strong> e-mail
              {coverage.emails.length > 1 ? "s" : ""} parent
              {coverage.withoutMail > 0
                ? ` · ${coverage.withoutMail} élève(s) sans mail`
                : ""}
              .
            </p>
          </div>

          {expired ? (
            <TripAlert tone="warning" title="Page publique fermée">
              Le délai de 15 jours après le retour est écoulé. Les familles n’y ont plus accès ;
              le contenu a été (ou va être) purgé.
            </TripAlert>
          ) : blog?.enabled ? (
            <div className="space-y-2">
              <p className="text-sm text-slate-700">
                Active depuis le{" "}
                {new Date(blog.activatedAt).toLocaleDateString("fr-FR")} · fermeture le{" "}
                {new Date(blog.expiresAt).toLocaleDateString("fr-FR")}
              </p>
              <div className="flex flex-wrap gap-2 items-center">
                <code className="text-xs bg-white/80 border border-violet-100 rounded px-2 py-1 break-all">
                  {publicUrl}
                </code>
                <TripButton variant="secondary" disabled={busy} onClick={() => void copyLink()}>
                  Copier le lien
                </TripButton>
                <a
                  href={blog.publicPath}
                  target="_blank"
                  rel="noreferrer"
                  className="text-sm font-semibold text-violet-800 underline"
                >
                  Ouvrir
                </a>
              </div>
            </div>
          ) : canEdit ? (
            <TripButton variant="primary" disabled={busy} onClick={() => void activateBlog()}>
              {busy ? "…" : "Activer la page de suivi + envoyer le mail"}
            </TripButton>
          ) : (
            <p className="text-sm text-slate-600">Page non activée.</p>
          )}
        </div>

        {blog?.enabled && !expired && canEdit ? (
          <div className="rounded-xl border border-slate-200 bg-white p-4 space-y-3">
            <h3 className="text-sm font-black text-slate-900">Délégation</h3>
            <p className="text-xs text-slate-600">
              Autorisez un autre professeur ou un personnel OGEC à publier sur cette page.
            </p>
            {delegates.length > 0 ? (
              <ul className="space-y-1">
                {delegates.map((d) => (
                  <li
                    key={d.userId}
                    className="flex items-center justify-between gap-2 text-sm rounded-lg bg-slate-50 px-3 py-2"
                  >
                    <span>
                      {d.name}
                      {d.email ? (
                        <span className="text-xs text-slate-500"> · {d.email}</span>
                      ) : null}
                    </span>
                    <button
                      type="button"
                      className="text-xs font-bold text-rose-600"
                      disabled={busy}
                      onClick={() => removeDelegate(d.userId)}
                    >
                      Retirer
                    </button>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-xs text-slate-500">Aucun délégué pour l’instant.</p>
            )}
            <div className="flex flex-wrap gap-2">
              <select
                className="rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-sm min-w-[220px]"
                value={delegatePick}
                disabled={busy}
                onChange={(e) => setDelegatePick(e.target.value)}
              >
                <option value="">Choisir une personne…</option>
                {directory.map((u) => (
                  <option key={u.externalUserId} value={u.externalUserId}>
                    {u.displayName}
                    {u.email ? ` (${u.email})` : ""}
                  </option>
                ))}
              </select>
              <TripButton
                variant="secondary"
                disabled={busy || !delegatePick}
                onClick={() => addDelegate()}
              >
                Ajouter
              </TripButton>
            </div>
          </div>
        ) : null}

        {blog?.enabled && !expired && canPublish ? (
          <div className="rounded-xl border border-amber-200 bg-amber-50/40 p-4 space-y-3">
            <h3 className="text-sm font-black text-amber-950">Nouvelle publication</h3>
            <TripTextarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={4}
              disabled={busy}
              placeholder="Ex. Nous sommes bien arrivés, la journée se passe bien…"
            />
            <div>
              <label className="block text-xs font-semibold text-slate-500 uppercase mb-2">
                Photos ({photos.length}/{MAX_PHOTOS})
              </label>
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
              {photos.length > 0 && (
                <div className="mt-3 flex flex-wrap gap-2">
                  {photos.map((p) => (
                    <div
                      key={p.id}
                      className="relative w-20 h-20 rounded-lg overflow-hidden ring-1 ring-slate-200"
                    >
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img src={p.previewUrl} alt="" className="w-full h-full object-cover" />
                      <button
                        type="button"
                        onClick={() => removePhoto(p.id)}
                        className="absolute top-0.5 right-0.5 bg-black/60 text-white text-[10px] rounded px-1"
                      >
                        ×
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
            <TripButton variant="primary" disabled={busy} onClick={() => void publish()}>
              {busy ? "Publication…" : "Publier sur la page parents"}
            </TripButton>
          </div>
        ) : null}

        {posts.length > 0 ? (
          <div className="pt-2 border-t border-slate-100">
            <h3 className="text-sm font-semibold text-slate-700 mb-2">Publications</h3>
            <ul className="space-y-3">
              {posts.map((p) => (
                <li key={p.id} className="rounded-lg bg-slate-50 px-3 py-3">
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <div className="text-sm font-medium text-slate-800">{p.authorName}</div>
                      <div className="text-xs text-slate-500">
                        {new Date(p.createdAt).toLocaleString("fr-FR")}
                      </div>
                    </div>
                    {canEdit && canPublish ? (
                      <button
                        type="button"
                        className="text-xs font-bold text-rose-600"
                        disabled={busy}
                        onClick={() => void deletePost(p.id)}
                      >
                        Supprimer
                      </button>
                    ) : null}
                  </div>
                  <p className="mt-2 text-sm whitespace-pre-wrap text-slate-700">{p.body}</p>
                  {p.photos.length > 0 ? (
                    <div className="mt-2 flex flex-wrap gap-2">
                      {p.photos.map((ph) =>
                        ph.url ? (
                          // eslint-disable-next-line @next/next/no-img-element
                          <img
                            key={ph.id}
                            src={ph.url}
                            alt=""
                            className="h-16 w-16 rounded object-cover ring-1 ring-slate-200"
                          />
                        ) : null,
                      )}
                    </div>
                  ) : null}
                </li>
              ))}
            </ul>
          </div>
        ) : null}
      </div>
    </TripSection>
  );
}
