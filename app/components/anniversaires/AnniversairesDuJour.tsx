"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { useAppUser } from "@/app/hooks/useAppUser";

type Data = {
  todayKey: string;
  eleves: { prenom: string; nom: string; classe: string }[];
  personnels: { prenom: string; nom: string; fonction: string }[];
};

const NON_INTERNES = new Set(["parent", "eleve"]);
const storageKey = (userId: string) => `scola-anniversaires:${userId}`;

function parisToday(): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());
  return parts; // YYYY-MM-DD
}

function readSeen(userId: string): string | null {
  try {
    return localStorage.getItem(storageKey(userId));
  } catch {
    return null;
  }
}
function writeSeen(userId: string, key: string) {
  try {
    localStorage.setItem(storageKey(userId), key);
  } catch {
    /* stockage indisponible : on retentera au prochain chargement */
  }
}

const COLORS = ["#e8d48b", "#f5e6a8", "#7eb8d4", "#f0c27b", "#f08fa8", "#9fd4c0", "#b79cf0"];

function ConfettiBurst() {
  const pieces = useMemo(
    () =>
      Array.from({ length: 60 }, (_, i) => ({
        id: i,
        left: (i * 37 + 11) % 100,
        delay: (i % 12) * 0.1,
        duration: 1.8 + (i % 6) * 0.25,
        size: 6 + (i % 5) * 2,
        rotate: (i * 47) % 360,
        color: COLORS[i % COLORS.length],
        drift: ((i % 5) - 2) * 22,
      })),
    [],
  );
  const [visible, setVisible] = useState(true);
  useEffect(() => {
    const t = window.setTimeout(() => setVisible(false), 3500);
    return () => window.clearTimeout(t);
  }, []);
  if (!visible) return null;
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden">
      {pieces.map((p) => (
        <span
          key={p.id}
          className="anniv-confetti absolute top-[-8%]"
          style={{
            left: `${p.left}%`,
            width: p.size,
            height: p.size * 1.6,
            background: p.color,
            borderRadius: p.size > 10 ? "2px" : "50%",
            animationDelay: `${p.delay}s`,
            animationDuration: `${p.duration}s`,
            ["--anniv-drift" as string]: `${p.drift}px`,
            ["--anniv-rot" as string]: `${p.rotate}deg`,
          }}
        />
      ))}
      <style>{`
        @keyframes anniv-confetti-fall {
          0% { opacity: 0; transform: translate3d(0, -10%, 0) rotate(0deg); }
          12% { opacity: 1; }
          100% { opacity: 0; transform: translate3d(var(--anniv-drift, 20px), 110vh, 0) rotate(var(--anniv-rot, 240deg)); }
        }
        .anniv-confetti {
          animation-name: anniv-confetti-fall;
          animation-timing-function: cubic-bezier(0.25, 0.7, 0.35, 1);
          animation-fill-mode: both;
        }
      `}</style>
    </div>
  );
}

export default function AnniversairesDuJour() {
  const { isLoaded, user, supervisionActive } = useAppUser();
  const [data, setData] = useState<Data | null>(null);
  const [open, setOpen] = useState(false);
  const [reduceMotion, setReduceMotion] = useState(true);
  const closeRef = useRef<HTMLButtonElement>(null);

  const userId = user?.businessUserId || user?.id || "";
  const eligible =
    isLoaded &&
    !!user?.isSignedIn &&
    !supervisionActive &&
    user.roles.some((r) => !NON_INTERNES.has(r));

  useEffect(() => {
    if (!eligible || !userId) return;
    const today = parisToday();
    if (readSeen(userId) === today) return;
    const ctrl = new AbortController();
    (async () => {
      try {
        const res = await fetch("/api/anniversaires", {
          credentials: "include",
          cache: "no-store",
          signal: ctrl.signal,
        });
        if (!res.ok) return;
        const json = (await res.json()) as Data;
        writeSeen(userId, json.todayKey || today);
        if ((json.eleves?.length ?? 0) + (json.personnels?.length ?? 0) > 0) {
          setReduceMotion(window.matchMedia("(prefers-reduced-motion: reduce)").matches);
          setData(json);
          setOpen(true);
        }
      } catch {
        /* silencieux : on retentera à la prochaine ouverture */
      }
    })();
    return () => ctrl.abort();
  }, [eligible, userId]);

  useEffect(() => {
    if (!open) return;
    closeRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open]);

  if (!open || !data) return null;

  return (
    <div
      className="fixed inset-0 z-[200] flex items-center justify-center bg-black/50 p-4"
      onClick={() => setOpen(false)}
    >
      {!reduceMotion ? <ConfettiBurst /> : null}
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="anniversaires-titre"
        className="relative flex max-h-[85vh] w-full max-w-md flex-col rounded-2xl bg-white p-5 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="anniversaires-titre" className="text-center text-xl font-bold text-slate-900">
          🎉 Anniversaires du jour
        </h2>
        <div className="mt-4 min-h-0 flex-1 space-y-4 overflow-y-auto">
          {data.eleves.length > 0 ? (
            <section>
              <h3 className="mb-1 text-sm font-semibold uppercase tracking-wide text-slate-500">
                Élèves
              </h3>
              <ul className="space-y-1">
                {data.eleves.map((e, i) => (
                  <li key={`e${i}`} className="rounded-lg bg-amber-50 px-3 py-2 text-sm text-slate-800">
                    🎂 {e.prenom} {e.nom}
                    {e.classe ? <span className="text-slate-500"> · {e.classe}</span> : null}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
          {data.personnels.length > 0 ? (
            <section>
              <h3 className="mb-1 text-sm font-semibold uppercase tracking-wide text-slate-500">
                Personnels
              </h3>
              <ul className="space-y-1">
                {data.personnels.map((p, i) => (
                  <li key={`p${i}`} className="rounded-lg bg-sky-50 px-3 py-2 text-sm text-slate-800">
                    🎂 {p.prenom} {p.nom}
                    {p.fonction ? <span className="text-slate-500"> · {p.fonction}</span> : null}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </div>
        <button
          ref={closeRef}
          type="button"
          onClick={() => setOpen(false)}
          className="mt-5 w-full rounded-xl bg-slate-900 px-4 py-2.5 text-sm font-semibold text-white hover:bg-slate-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-amber-400"
        >
          Fermer
        </button>
      </div>
    </div>
  );
}
