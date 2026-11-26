"use client";

/**
 * Fil PRIVÉ d'un voyage, côté familles (lien personnel, sans compte) : lecture + réactions (👍 ❤️ 😄 👏).
 * Pas de commentaires, pas de messages : les familles écrivent à l'établissement par ses voies habituelles.
 */

import { useCallback, useEffect, useRef, useState } from "react";

type FeedPost = {
  id: string;
  kind: string;
  text: string;
  createdAt: string;
  photos: { id: string; url: string; width: number; height: number }[];
  reactions: Record<string, number>;
  mine: string[];
};

type FeedView = {
  tripTitle: string;
  destination: string;
  readOnly: boolean;
  deletedOn: string | null;
  allowDownload: boolean;
  reactionEmojis: string[];
  posts: FeedPost[];
};

const KIND_ICON: Record<string, string> = {
  arrives: "📍",
  ok: "✅",
  delay: "⏱️",
  departure_back: "🚌",
  at_school: "🏫",
  program: "🗓️",
  message: "💬",
  photos: "📷",
};

const REFRESH_MS = 90_000;

function whenFr(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", { timeZone: "Europe/Paris", weekday: "short", day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

function dayFr(iso: string): string {
  return new Date(`${iso}T12:00:00Z`).toLocaleDateString("fr-FR", { timeZone: "Europe/Paris", day: "numeric", month: "long", year: "numeric" });
}

export function FamilyFeedPage({ token }: { token: string }) {
  const [view, setView] = useState<FeedView | null>(null);
  const [invalid, setInvalid] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [zoom, setZoom] = useState<{ url: string; w: number; h: number } | null>(null);
  const busy = useRef(new Set<string>());
  const base = `/api/voyages/famille/fil/${encodeURIComponent(token)}`;

  const load = useCallback(async () => {
    try {
      const res = await fetch(base, { cache: "no-store", referrerPolicy: "no-referrer" });
      if (res.status === 404) {
        setInvalid(true);
        return;
      }
      if (!res.ok) throw new Error("Impossible de charger le fil.");
      setView((await res.json()) as FeedView);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Impossible de charger le fil.");
    }
  }, [base]);

  useEffect(() => {
    void load();
    const t = setInterval(() => {
      if (document.visibilityState === "visible") void load();
    }, REFRESH_MS);
    return () => clearInterval(t);
  }, [load]);

  const react = async (post: FeedPost, emoji: string) => {
    const key = `${post.id}${emoji}`;
    if (!view || view.readOnly || busy.current.has(key)) return;
    busy.current.add(key);
    const on = !post.mine.includes(emoji);
    // Affichage immédiat, confirmé ensuite par le serveur.
    setView({
      ...view,
      posts: view.posts.map((p) =>
        p.id !== post.id
          ? p
          : {
              ...p,
              mine: on ? [...p.mine, emoji] : p.mine.filter((e) => e !== emoji),
              reactions: { ...p.reactions, [emoji]: Math.max(0, (p.reactions[emoji] || 0) + (on ? 1 : -1)) },
            },
      ),
    });
    try {
      const res = await fetch(`${base}/react`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        referrerPolicy: "no-referrer",
        body: JSON.stringify({ postId: post.id, emoji, on }),
      });
      if (!res.ok) throw new Error();
    } catch {
      /* resynchronisation ci-dessous */
    } finally {
      busy.current.delete(key);
      void load();
    }
  };

  if (invalid) {
    return (
      <main className="mx-auto max-w-xl p-4 sm:p-8">
        <div className="rounded-2xl border-2 border-amber-200 bg-amber-50 p-6 text-sm text-amber-950">
          <h1 className="text-lg font-black">Lien invalide ou expiré</h1>
          <p className="mt-2">
            Ce fil est privé et réservé aux familles des élèves participants. Si le voyage est terminé depuis plus d&apos;une semaine, le fil a été supprimé. Sinon, utilisez le lien du
            dernier e-mail reçu.
          </p>
        </div>
      </main>
    );
  }
  if (error && !view) return <main className="mx-auto max-w-xl p-8 text-sm font-semibold text-rose-700">{error}</main>;
  if (!view) return <main className="mx-auto max-w-xl p-8 text-sm text-slate-500">Chargement…</main>;

  return (
    <main className="mx-auto max-w-xl space-y-4 p-4 pb-16 sm:p-6">
      <header className="rounded-3xl bg-gradient-to-br from-indigo-600 to-violet-600 p-5 text-white shadow-lg">
        <p className="text-xs font-bold uppercase tracking-widest text-indigo-100">Fil du voyage</p>
        <h1 className="mt-1 text-xl font-black leading-tight">{view.tripTitle}</h1>
        {view.destination ? <p className="mt-0.5 text-sm text-indigo-100">{view.destination}</p> : null}
      </header>

      {view.readOnly ? (
        <p className="rounded-2xl border border-slate-200 bg-slate-50 p-3 text-sm text-slate-700" role="status">
          Le voyage est terminé : le fil est en lecture seule.
          {view.deletedOn ? ` Il sera supprimé définitivement le ${dayFr(view.deletedOn)}.` : ""}
        </p>
      ) : (
        <p className="px-1 text-xs text-slate-500">
          Fil privé, réservé aux familles des élèves participants. Ne partagez pas ce lien.
          {view.deletedOn ? ` Tout sera supprimé le ${dayFr(view.deletedOn)}.` : ""}
        </p>
      )}

      {view.posts.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-slate-300 p-8 text-center text-sm text-slate-500">Rien de publié pour l&apos;instant. Les nouvelles du groupe apparaîtront ici.</div>
      ) : (
        <ul className="space-y-3">
          {view.posts.map((p) => (
            <li key={p.id} className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm">
              <div className="flex items-start gap-3">
                <span className="text-2xl" aria-hidden>
                  {KIND_ICON[p.kind] || "💬"}
                </span>
                <div className="min-w-0 flex-1">
                  <p className="text-xs font-semibold text-slate-400">{whenFr(p.createdAt)}</p>
                  {p.text ? <p className="mt-1 whitespace-pre-line break-words text-[15px] font-semibold text-slate-900">{p.text}</p> : null}
                </div>
              </div>

              {p.photos.length > 0 ? (
                <div className={`mt-3 grid gap-1.5 ${p.photos.length === 1 ? "grid-cols-1" : "grid-cols-2"}`}>
                  {p.photos.map((ph) => (
                    <div key={ph.id} className="relative overflow-hidden rounded-xl bg-slate-100">
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img
                        src={ph.url}
                        alt="Photo du voyage"
                        loading="lazy"
                        draggable={false}
                        referrerPolicy="no-referrer"
                        onContextMenu={view.allowDownload ? undefined : (e) => e.preventDefault()}
                        onClick={() => setZoom({ url: ph.url, w: ph.width, h: ph.height })}
                        className="aspect-[4/3] w-full cursor-zoom-in select-none object-cover"
                      />
                      {view.allowDownload ? (
                        <a href={ph.url} download className="absolute bottom-1.5 right-1.5 rounded-lg bg-white/90 px-2 py-1 text-xs font-bold text-slate-800 shadow">
                          Télécharger
                        </a>
                      ) : null}
                    </div>
                  ))}
                </div>
              ) : null}

              <div className="mt-3 flex flex-wrap gap-2">
                {view.reactionEmojis.map((emoji) => {
                  const mine = p.mine.includes(emoji);
                  const n = p.reactions[emoji] || 0;
                  return (
                    <button
                      key={emoji}
                      type="button"
                      disabled={view.readOnly}
                      onClick={() => void react(p, emoji)}
                      aria-pressed={mine}
                      aria-label={`${emoji} ${n}`}
                      className={`flex min-h-[40px] items-center gap-1.5 rounded-full border px-3 text-sm font-bold transition ${
                        mine ? "border-indigo-400 bg-indigo-50 text-indigo-800" : "border-slate-200 bg-white text-slate-600"
                      } ${view.readOnly ? "opacity-70" : "active:scale-95"}`}
                    >
                      <span className="text-base">{emoji}</span>
                      {n > 0 ? <span>{n}</span> : null}
                    </button>
                  );
                })}
              </div>
            </li>
          ))}
        </ul>
      )}

      {zoom ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/90 p-3" role="dialog" aria-modal="true" onClick={() => setZoom(null)}>
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src={zoom.url}
            alt="Photo du voyage"
            draggable={false}
            referrerPolicy="no-referrer"
            onContextMenu={view.allowDownload ? undefined : (e) => e.preventDefault()}
            className="max-h-full max-w-full select-none rounded-lg object-contain"
          />
          <button type="button" className="absolute right-3 top-3 rounded-full bg-white/90 px-4 py-2 text-sm font-black text-slate-900" onClick={() => setZoom(null)}>
            Fermer
          </button>
        </div>
      ) : null}
    </main>
  );
}
