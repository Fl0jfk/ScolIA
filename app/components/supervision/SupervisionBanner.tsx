"use client";

import { useCallback, useEffect, useState } from "react";

type SupervisionStatus = {
  active: boolean;
  target?: {
    userId: string;
    email: string;
    displayName: string;
    roles: string[];
  };
};

/** Bandeau fixe : mode supervision lecture seule. */
export default function SupervisionBanner() {
  const [status, setStatus] = useState<SupervisionStatus | null>(null);
  const [stopping, setStopping] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const res = await fetch("/api/supervision/status", { cache: "no-store" });
      if (!res.ok) {
        setStatus({ active: false });
        return;
      }
      const j = (await res.json()) as SupervisionStatus;
      setStatus(j);
    } catch {
      setStatus({ active: false });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const stop = async () => {
    setStopping(true);
    try {
      await fetch("/api/supervision/stop", { method: "POST" });
      window.location.assign("/parametres?tab=utilisateurs");
    } catch {
      setStopping(false);
    }
  };

  if (!status?.active || !status.target) return null;

  return (
    <div
      role="status"
      className="sticky top-0 z-[60] border-b border-amber-300/80 bg-amber-50 text-amber-950 shadow-sm"
    >
      <div className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-3 px-4 py-2.5 sm:px-6">
        <div className="flex min-w-0 items-start gap-2.5">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            strokeWidth={2}
            stroke="currentColor"
            className="mt-0.5 h-5 w-5 shrink-0 text-amber-700"
            aria-hidden
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z"
            />
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"
            />
          </svg>
          <div className="min-w-0">
            <p className="text-sm font-semibold leading-snug">
              Supervision — lecture seule
            </p>
            <p className="truncate text-xs text-amber-900/80">
              Vous consultez l’espace de{" "}
              <span className="font-semibold">{status.target.displayName}</span>
              {status.target.email ? (
                <>
                  {" "}
                  <span className="text-amber-800/70">({status.target.email})</span>
                </>
              ) : null}
              . Aucune modification n’est possible.
            </p>
          </div>
        </div>
        <button
          type="button"
          onClick={() => void stop()}
          disabled={stopping}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-xl border border-amber-400/70 bg-white px-3 py-1.5 text-sm font-semibold text-amber-950 shadow-sm transition hover:bg-amber-100 disabled:opacity-60"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            strokeWidth={2}
            stroke="currentColor"
            className="h-4 w-4"
            aria-hidden
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
          {stopping ? "Sortie…" : "Quitter"}
        </button>
      </div>
    </div>
  );
}
