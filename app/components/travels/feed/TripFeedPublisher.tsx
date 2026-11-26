"use client";

/**
 * Publication du « fil du voyage » (profs) — pensée pour le téléphone : raccourcis en un clic, programme, message court,
 * photos rapides. Les familles reçoivent un e-mail regroupé (au plus un par heure) reprenant le TEXTE des publications.
 */

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { compressPhotoFile, type CompressedPhoto } from "@/app/lib/client-image-compress";

type StaffPost = {
  id: string;
  kind: string;
  text: string;
  createdAt: string;
  photos: { id: string; url: string }[];
  reactions: Record<string, number>;
  hidden: boolean;
  authorName: string | null;
};

type StaffView = {
  opened: boolean;
  phase: "unavailable" | "live" | "readonly" | "purge_due";
  canPublish: boolean;
  canModerate: boolean;
  tripTitle: string;
  allowDownload: boolean;
  imageRightList: { ine: string; nom: string; prenom: string; classe?: string; status: "refuse" | "non_renseigne" }[];
  participantCount: number;
  families: { total: number; linkSent: number; failures: number };
  emailsSent: number;
  emailCap: number;
  deletedOn: string | null;
  posts: StaffPost[];
  audit: { action: string; actorName: string | null; at: string; detail: string | null }[];
};

const SHORTCUTS: { kind: string; label: string; icon: string; tone: string }[] = [
  { kind: "arrives", label: "Bien arrivés", icon: "📍", tone: "bg-emerald-600" },
  { kind: "ok", label: "Tout va bien", icon: "✅", tone: "bg-sky-600" },
  { kind: "departure_back", label: "Départ du retour", icon: "🚌", tone: "bg-indigo-600" },
  { kind: "at_school", label: "Arrivés à l'établissement", icon: "🏫", tone: "bg-violet-600" },
];

const DELAYS = [10, 15, 20, 30, 45, 60];

const AUDIT_LABELS: Record<string, string> = {
  feed_opened: "Fil ouvert",
  feed_links_resent: "Liens renvoyés",
  photos_published: "Photos publiées",
  post_hidden: "Publication masquée",
  post_unhidden: "Publication rétablie",
  post_deleted: "Publication supprimée",
  download_enabled: "Téléchargement autorisé",
  download_disabled: "Téléchargement désactivé",
  feed_purged: "Fil supprimé",
};

const WARNING =
  "Ce texte est envoyé par e-mail à TOUTES les familles des élèves participants. N'y mettez aucune information personnelle sur un élève (nom, santé, comportement…).";

function when(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", { timeZone: "Europe/Paris", day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

export function TripFeedPublisher({ tripId }: { tripId: string }) {
  const [view, setView] = useState<StaffView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [program, setProgram] = useState("");
  const [showDelay, setShowDelay] = useState(false);
  const [photos, setPhotos] = useState<CompressedPhoto[]>([]);
  const [caption, setCaption] = useState("");
  const [ack, setAck] = useState(false);
  const cameraRef = useRef<HTMLInputElement>(null);
  const galleryRef = useRef<HTMLInputElement>(null);
  const base = `/api/travels/feed/${encodeURIComponent(tripId)}`;

  const load = useCallback(async () => {
    try {
      const res = await fetch(base, { cache: "no-store" });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Chargement impossible.");
      setView(j as StaffView);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement impossible.");
    }
  }, [base]);

  useEffect(() => {
    void load();
  }, [load]);

  const say = (m: string) => {
    setToast(m);
    setTimeout(() => setToast(null), 4000);
  };

  const post = async (body: Record<string, unknown>, okMsg: string): Promise<boolean> => {
    setBusy(true);
    try {
      const res = await fetch(base, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Action impossible.");
      say(body.action === "open" ? `${okMsg} (${j.families} famille(s), ${j.sent} lien(s) envoyé(s)${j.failed ? `, ${j.failed} échec(s)` : ""}${j.withoutAddress ? `, ${j.withoutAddress} élève(s) sans adresse` : ""})` : okMsg);
      await load();
      return true;
    } catch (e) {
      say(e instanceof Error ? e.message : "Action impossible.");
      return false;
    } finally {
      setBusy(false);
    }
  };

  const addFiles = async (files: FileList | null) => {
    if (!files?.length) return;
    setBusy(true);
    try {
      const next = [...photos];
      for (const f of Array.from(files)) {
        if (next.length >= 10) break;
        next.push(await compressPhotoFile(f));
      }
      setPhotos(next);
    } catch (e) {
      say(e instanceof Error ? e.message : "Photo illisible.");
    } finally {
      setBusy(false);
      if (cameraRef.current) cameraRef.current.value = "";
      if (galleryRef.current) galleryRef.current.value = "";
    }
  };

  const sendPhotos = async () => {
    if (photos.length === 0) return;
    setBusy(true);
    try {
      const fd = new FormData();
      for (const p of photos) fd.append("files", p.blob, p.name);
      fd.append("ack", ack ? "1" : "0");
      if (caption.trim()) fd.append("caption", caption.trim());
      const res = await fetch(`${base}/photos`, { method: "POST", body: fd });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Envoi impossible.");
      photos.forEach((p) => URL.revokeObjectURL(p.previewUrl));
      setPhotos([]);
      setCaption("");
      setAck(false);
      say("Photos publiées.");
      await load();
    } catch (e) {
      say(e instanceof Error ? e.message : "Envoi impossible.");
    } finally {
      setBusy(false);
    }
  };

  if (error && !view) {
    return (
      <main className="mx-auto max-w-xl p-6">
        <p className="text-sm font-semibold text-rose-700">{error}</p>
        <Link href={`/travels/${tripId}`} className="mt-3 inline-block text-sm font-bold text-indigo-700 underline">
          Retour au voyage
        </Link>
      </main>
    );
  }
  if (!view) return <main className="mx-auto max-w-xl p-6 text-sm text-slate-500">Chargement…</main>;

  const live = view.phase === "live";
  const canPost = view.canPublish && view.opened && live;
  const needAck = view.imageRightList.length > 0;

  return (
    <main className="mx-auto max-w-xl space-y-4 p-3 pb-24 sm:p-6">
      <div className="flex items-center justify-between gap-2">
        <Link href={`/travels/${tripId}`} className="text-sm font-bold text-indigo-700">
          ← Voyage
        </Link>
        <span className={`rounded-full px-3 py-1 text-xs font-black ${live ? "bg-emerald-100 text-emerald-800" : "bg-slate-200 text-slate-700"}`}>
          {!view.opened ? "Non ouvert" : live ? "En direct" : "Lecture seule"}
        </span>
      </div>
      <header className="rounded-3xl bg-gradient-to-br from-indigo-600 to-violet-600 p-5 text-white shadow-lg">
        <p className="text-xs font-bold uppercase tracking-widest text-indigo-100">Fil du voyage — privé</p>
        <h1 className="mt-1 text-xl font-black leading-tight">{view.tripTitle}</h1>
        <p className="mt-1 text-xs text-indigo-100">
          {view.opened
            ? `${view.families.linkSent}/${view.families.total} familles ont reçu leur lien · ${view.emailsSent} e-mail(s) de nouveautés envoyé(s)`
            : "Réservé aux familles des élèves participants (lien personnel)."}
          {view.deletedOn ? ` Tout sera supprimé le ${new Date(`${view.deletedOn}T12:00:00Z`).toLocaleDateString("fr-FR")}.` : ""}
        </p>
      </header>

      {toast ? (
        <div className="sticky top-2 z-20 rounded-2xl bg-slate-900 px-4 py-3 text-sm font-bold text-white shadow-xl" role="status">
          {toast}
        </div>
      ) : null}

      {view.phase === "unavailable" || view.phase === "purge_due" ? (
        <p className="rounded-2xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">Le fil n&apos;est pas disponible pour ce voyage (annulé, sans date de retour ou terminé).</p>
      ) : null}

      {!view.opened && live ? (
        <section className="space-y-3 rounded-2xl border-2 border-indigo-200 bg-indigo-50 p-4">
          <h2 className="font-black text-indigo-950">Ouvrir le fil aux familles</h2>
          <p className="text-sm text-indigo-900">
            Chaque responsable des {view.participantCount} élève(s) participants reçoit un e-mail avec son lien personnel. Rien n&apos;est public, aucun compte n&apos;est nécessaire.
          </p>
          {view.canModerate ? (
            <button type="button" disabled={busy} onClick={() => void post({ action: "open" }, "Fil ouvert")} className="min-h-[48px] w-full rounded-xl bg-indigo-700 px-4 text-base font-black text-white disabled:opacity-50">
              Ouvrir le fil et envoyer les liens
            </button>
          ) : (
            <p className="text-sm font-semibold text-indigo-900">Seuls l&apos;organisateur et la direction peuvent ouvrir le fil.</p>
          )}
        </section>
      ) : null}

      {canPost ? (
        <>
          <section className="space-y-2">
            <h2 className="px-1 text-sm font-black uppercase tracking-wide text-slate-500">En un clic</h2>
            <div className="grid grid-cols-2 gap-2">
              {SHORTCUTS.map((s) => (
                <button
                  key={s.kind}
                  type="button"
                  disabled={busy}
                  onClick={() => void post({ action: "publish", kind: s.kind }, `Publié : ${s.label}`)}
                  className={`flex min-h-[72px] flex-col items-center justify-center gap-1 rounded-2xl ${s.tone} px-2 py-3 text-center text-sm font-black text-white shadow active:scale-95 disabled:opacity-50`}
                >
                  <span className="text-2xl" aria-hidden>
                    {s.icon}
                  </span>
                  {s.label}
                </button>
              ))}
              <button
                type="button"
                disabled={busy}
                onClick={() => setShowDelay((v) => !v)}
                className="col-span-2 flex min-h-[56px] items-center justify-center gap-2 rounded-2xl bg-amber-500 px-2 py-3 text-sm font-black text-white shadow active:scale-95 disabled:opacity-50"
              >
                <span className="text-2xl" aria-hidden>
                  ⏱️
                </span>
                Petit retard
              </button>
            </div>
            {showDelay ? (
              <div className="flex flex-wrap gap-2 rounded-2xl border border-amber-200 bg-amber-50 p-3">
                {DELAYS.map((m) => (
                  <button
                    key={m}
                    type="button"
                    disabled={busy}
                    onClick={() => {
                      setShowDelay(false);
                      void post({ action: "publish", kind: "delay", delayMinutes: m }, `Publié : petit retard (~${m} min)`);
                    }}
                    className="min-h-[44px] rounded-full border border-amber-300 bg-white px-4 text-sm font-black text-amber-900 active:scale-95"
                  >
                    ~{m} min
                  </button>
                ))}
              </div>
            ) : null}
          </section>

          <section className="space-y-2 rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-black text-slate-900">Programme du jour</h2>
            <textarea value={program} onChange={(e) => setProgram(e.target.value)} maxLength={1500} rows={4} placeholder="9 h visite du musée, 12 h pique-nique, 14 h plage…" className="w-full rounded-xl border border-slate-300 p-3 text-base" />
            <p className="text-xs text-amber-800">{WARNING}</p>
            <button
              type="button"
              disabled={busy || !program.trim()}
              onClick={async () => {
                if (await post({ action: "publish", kind: "program", body: program }, "Programme publié")) setProgram("");
              }}
              className="min-h-[44px] w-full rounded-xl bg-slate-900 px-4 text-sm font-black text-white disabled:opacity-40"
            >
              Publier le programme
            </button>
          </section>

          <section className="space-y-2 rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-black text-slate-900">Message court</h2>
            <textarea value={message} onChange={(e) => setMessage(e.target.value)} maxLength={500} rows={3} placeholder="Une nouvelle pour les familles…" className="w-full rounded-xl border border-slate-300 p-3 text-base" />
            <p className="text-right text-xs text-slate-400">{message.length}/500</p>
            <p className="text-xs text-amber-800">{WARNING}</p>
            <button
              type="button"
              disabled={busy || !message.trim()}
              onClick={async () => {
                if (await post({ action: "publish", kind: "message", body: message }, "Message publié")) setMessage("");
              }}
              className="min-h-[44px] w-full rounded-xl bg-slate-900 px-4 text-sm font-black text-white disabled:opacity-40"
            >
              Publier le message
            </button>
          </section>

          <section className="space-y-3 rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-black text-slate-900">Photos</h2>
            <div className="grid grid-cols-2 gap-2">
              <button type="button" disabled={busy} onClick={() => cameraRef.current?.click()} className="min-h-[52px] rounded-xl bg-rose-600 px-3 text-sm font-black text-white active:scale-95 disabled:opacity-50">
                📸 Prendre une photo
              </button>
              <button type="button" disabled={busy} onClick={() => galleryRef.current?.click()} className="min-h-[52px] rounded-xl bg-slate-800 px-3 text-sm font-black text-white active:scale-95 disabled:opacity-50">
                🖼️ Galerie
              </button>
            </div>
            <input ref={cameraRef} type="file" accept="image/*" capture="environment" className="hidden" onChange={(e) => void addFiles(e.target.files)} />
            <input ref={galleryRef} type="file" accept="image/jpeg,image/png,image/webp,image/heic,image/heif" multiple className="hidden" onChange={(e) => void addFiles(e.target.files)} />
            {photos.length > 0 ? (
              <>
                <div className="grid grid-cols-4 gap-1.5">
                  {photos.map((p) => (
                    <div key={p.id} className="relative aspect-square overflow-hidden rounded-lg bg-slate-100">
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img src={p.previewUrl} alt="" className="h-full w-full object-cover" />
                      <button
                        type="button"
                        aria-label="Retirer cette photo"
                        onClick={() => {
                          URL.revokeObjectURL(p.previewUrl);
                          setPhotos(photos.filter((x) => x.id !== p.id));
                        }}
                        className="absolute right-0.5 top-0.5 h-6 w-6 rounded-full bg-black/70 text-xs font-black text-white"
                      >
                        ×
                      </button>
                    </div>
                  ))}
                </div>
                {needAck ? (
                  <div className="space-y-2 rounded-xl border-2 border-rose-300 bg-rose-50 p-3">
                    <p className="text-sm font-black text-rose-900">Droit à l&apos;image : ces élèves ne doivent pas être reconnaissables</p>
                    <ul className="max-h-40 space-y-0.5 overflow-auto text-sm text-rose-950">
                      {view.imageRightList.map((s) => (
                        <li key={s.ine}>
                          {s.prenom} {s.nom}
                          {s.classe ? ` (${s.classe})` : ""} — <span className="font-bold">{s.status === "refuse" ? "droit à l'image refusé" : "non renseigné"}</span>
                        </li>
                      ))}
                    </ul>
                    <label className="flex items-start gap-2 text-sm font-bold text-rose-950">
                      <input type="checkbox" checked={ack} onChange={(e) => setAck(e.target.checked)} className="mt-0.5 h-5 w-5" />
                      Aucun de ces élèves n&apos;est reconnaissable sur ces photos
                    </label>
                  </div>
                ) : null}
                <input value={caption} onChange={(e) => setCaption(e.target.value)} maxLength={500} placeholder="Légende (facultatif, visible sur la page, jamais dans l'e-mail)" className="w-full rounded-xl border border-slate-300 p-3 text-sm" />
                <button type="button" disabled={busy || (needAck && !ack)} onClick={() => void sendPhotos()} className="min-h-[48px] w-full rounded-xl bg-rose-700 px-4 text-base font-black text-white disabled:opacity-40">
                  Publier {photos.length} photo{photos.length > 1 ? "s" : ""}
                </button>
                <p className="text-xs text-slate-500">Les familles sont prévenues par e-mail, sans les photos : elles les voient sur leur page privée.</p>
              </>
            ) : null}
          </section>
        </>
      ) : null}

      {view.opened ? (
        <section className="space-y-2">
          <h2 className="px-1 text-sm font-black uppercase tracking-wide text-slate-500">Publications</h2>
          {view.posts.length === 0 ? <p className="rounded-2xl border border-dashed border-slate-300 p-6 text-center text-sm text-slate-500">Rien de publié.</p> : null}
          <ul className="space-y-2">
            {view.posts.map((p) => (
              <li key={p.id} className={`rounded-2xl border p-3 ${p.hidden ? "border-rose-200 bg-rose-50/60" : "border-slate-200 bg-white"}`}>
                <p className="text-xs text-slate-400">
                  {when(p.createdAt)}
                  {p.authorName ? ` — ${p.authorName}` : ""}
                  {p.hidden ? " — masquée" : ""}
                </p>
                {p.text ? <p className="mt-1 whitespace-pre-line break-words text-sm font-semibold text-slate-900">{p.text}</p> : null}
                {p.photos.length > 0 ? (
                  <div className="mt-2 grid grid-cols-4 gap-1">
                    {p.photos.map((ph) => (
                      // eslint-disable-next-line @next/next/no-img-element
                      <img key={ph.id} src={ph.url} alt="" className="aspect-square rounded-lg object-cover" referrerPolicy="no-referrer" />
                    ))}
                  </div>
                ) : null}
                {Object.keys(p.reactions).length > 0 ? (
                  <p className="mt-2 text-sm">
                    {Object.entries(p.reactions)
                      .filter(([, n]) => n > 0)
                      .map(([e, n]) => `${e} ${n}`)
                      .join("  ")}
                  </p>
                ) : null}
                {view.canModerate ? (
                  <div className="mt-2 flex gap-2">
                    <button type="button" disabled={busy} onClick={() => void post({ action: "moderate", postId: p.id, op: p.hidden ? "unhide" : "hide" }, p.hidden ? "Publication rétablie" : "Publication masquée")} className="min-h-[36px] rounded-lg border border-slate-300 px-3 text-xs font-bold text-slate-700">
                      {p.hidden ? "Rétablir" : "Masquer"}
                    </button>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => {
                        if (window.confirm("Supprimer définitivement cette publication (et ses photos) ?")) void post({ action: "moderate", postId: p.id, op: "delete" }, "Publication supprimée");
                      }}
                      className="min-h-[36px] rounded-lg border border-rose-300 px-3 text-xs font-bold text-rose-700"
                    >
                      Supprimer
                    </button>
                  </div>
                ) : null}
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {view.opened && view.canModerate ? (
        <section className="space-y-3 rounded-2xl border border-slate-200 bg-white p-4">
          <h2 className="font-black text-slate-900">Gestion du fil</h2>
          {live ? (
            <button type="button" disabled={busy} onClick={() => void post({ action: "open" }, "Liens renvoyés")} className="min-h-[40px] w-full rounded-xl border border-indigo-300 px-3 text-sm font-bold text-indigo-800">
              Envoyer le lien aux familles qui ne l&apos;ont pas reçu ({view.families.total - view.families.linkSent})
            </button>
          ) : null}
          <label className="flex items-start gap-2 text-sm text-slate-700">
            <input type="checkbox" checked={view.allowDownload} disabled={busy} onChange={(e) => void post({ action: "download", allow: e.target.checked }, e.target.checked ? "Téléchargement autorisé" : "Téléchargement désactivé")} className="mt-0.5 h-5 w-5" />
            Autoriser les familles à télécharger les photos (désactivé par défaut)
          </label>
          {view.audit.length > 0 ? (
            <details>
              <summary className="cursor-pointer text-sm font-bold text-slate-600">Journal ({view.audit.length})</summary>
              <ul className="mt-2 space-y-1 text-xs text-slate-600">
                {view.audit.map((a, i) => (
                  <li key={i}>
                    {when(a.at)} — {AUDIT_LABELS[a.action] || a.action}
                    {a.actorName ? ` — ${a.actorName}` : ""}
                  </li>
                ))}
              </ul>
            </details>
          ) : null}
        </section>
      ) : null}
    </main>
  );
}
