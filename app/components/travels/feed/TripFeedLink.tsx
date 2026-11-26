"use client";

import { useEffect, useState } from "react";

/** Lien vers la page de publication du fil : affiché seulement à ceux qui peuvent publier ou modérer sur CE voyage (contrôle serveur). */
export function TripFeedLink({ tripId }: { tripId: string }) {
  const [allowed, setAllowed] = useState(false);
  useEffect(() => {
    let cancelled = false;
    fetch(`/api/travels/feed/${encodeURIComponent(tripId)}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ action: "access" }),
    })
      .then((r) => (r.ok ? r.json() : null))
      .then((j) => {
        if (!cancelled) setAllowed(Boolean(j?.allowed));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [tripId]);
  if (!allowed) return null;
  return (
    <a href={`/travels/${tripId}/fil`} className="inline-flex min-h-[40px] items-center rounded-xl bg-white/90 px-3 text-sm font-black text-indigo-800 shadow">
      📣 Fil du voyage
    </a>
  );
}
