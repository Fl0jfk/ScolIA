"use client";

import { useCallback, useEffect, useState } from "react";
import { useParams } from "next/navigation";

type PublicPost = {
  id: string;
  authorName: string;
  body: string;
  createdAt: string;
  photos: Array<{ id: string; url: string }>;
};

type PublicBlog = {
  title: string;
  destination: string | null;
  startDate: string | null;
  endDate: string | null;
  expiresAt: string;
  posts: PublicPost[];
};

function formatDates(start: string | null, end: string | null): string {
  if (!start && !end) return "";
  const fmt = (ymd: string) => {
    try {
      return new Date(`${ymd}T12:00:00`).toLocaleDateString("fr-FR", {
        day: "numeric",
        month: "long",
        year: "numeric",
      });
    } catch {
      return ymd;
    }
  };
  if (start && end && start !== end) return `${fmt(start)} → ${fmt(end)}`;
  return fmt(start || end || "");
}

export default function VoyageSuiviPublicPage() {
  const params = useParams();
  const token = typeof params.token === "string" ? params.token : "";
  const [data, setData] = useState<PublicBlog | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [expired, setExpired] = useState(false);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!token) {
      setError("Lien incomplet.");
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const res = await fetch(`/api/travels/public-blog?token=${encodeURIComponent(token)}`, {
        cache: "no-store",
      });
      const j = await res.json();
      if (res.status === 410) {
        setExpired(true);
        setError(j.error || "Cette page est fermée.");
        setData(null);
        return;
      }
      if (!res.ok) throw new Error(j.error || "Page introuvable");
      setData(j as PublicBlog);
      setError(null);
      setExpired(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur de chargement");
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [token]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="min-h-screen bg-slate-50 text-slate-800">
      <div className="mx-auto max-w-2xl px-4 py-10 sm:py-14">
        {loading ? (
          <p className="text-sm text-slate-500">Chargement…</p>
        ) : error ? (
          <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
            <h1 className="text-xl font-semibold text-slate-900">
              {expired ? "Page fermée" : "Suivi indisponible"}
            </h1>
            <p className="mt-3 text-sm leading-relaxed text-slate-600">{error}</p>
          </div>
        ) : data ? (
          <>
            <header className="mb-8">
              <p className="text-xs font-semibold uppercase tracking-wider text-sky-700">
                Suivi de sortie
              </p>
              <h1 className="mt-2 text-3xl font-semibold tracking-tight text-slate-900">
                {data.title}
              </h1>
              {data.destination ? (
                <p className="mt-1 text-base text-slate-600">{data.destination}</p>
              ) : null}
              {formatDates(data.startDate, data.endDate) ? (
                <p className="mt-2 text-sm text-slate-500">
                  {formatDates(data.startDate, data.endDate)}
                </p>
              ) : null}
              <p className="mt-4 text-xs text-slate-500">
                Page en lecture seule · fermeture automatique 15 jours après le retour
              </p>
            </header>

            {data.posts.length === 0 ? (
              <div className="rounded-2xl border border-dashed border-slate-200 bg-white px-6 py-10 text-center text-sm text-slate-500">
                Aucune publication pour le moment. Revenez plus tard.
              </div>
            ) : (
              <ul className="space-y-6">
                {data.posts.map((post) => (
                  <li
                    key={post.id}
                    className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm"
                  >
                    <div className="mb-3 flex flex-wrap items-baseline justify-between gap-2">
                      <span className="text-sm font-semibold text-slate-800">
                        {post.authorName || "Équipe"}
                      </span>
                      <time className="text-xs text-slate-500">
                        {new Date(post.createdAt).toLocaleString("fr-FR")}
                      </time>
                    </div>
                    <p className="whitespace-pre-wrap text-sm leading-relaxed text-slate-700">
                      {post.body}
                    </p>
                    {post.photos.length > 0 ? (
                      <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3">
                        {post.photos.map((ph) => (
                          // eslint-disable-next-line @next/next/no-img-element
                          <img
                            key={ph.id}
                            src={ph.url}
                            alt=""
                            className="aspect-square w-full rounded-lg object-cover"
                          />
                        ))}
                      </div>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </>
        ) : null}
      </div>
    </div>
  );
}
