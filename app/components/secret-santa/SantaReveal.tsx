"use client";

import { useEffect, useRef, useState } from "react";
import SantaGift, { GIFT_SHAKE_MS, type GiftPhase } from "@/app/components/secret-santa/SantaGift";

/**
 * Le moment du tirage : compte à rebours (calé sur l'heure du serveur), puis cadeau qui se secoue,
 * éclate et laisse sortir la tête de la personne tirée. Le nom n'arrive du serveur qu'à partir de l'heure prévue.
 */

const COLORS = ["#fca5a5", "#fde68a", "#86efac", "#ffffff", "#fbcfe8", "#93c5fd"];

/** Positions calculées à partir de l'indice : rendu stable, sans tirage au rendu. */
const SNOW = Array.from({ length: 40 }, (_, i) => ({
  left: (i * 41 + 7) % 100,
  delay: -((i * 13) % 90) / 10,
  duration: 7 + (i % 6),
  size: 3 + (i % 4) * 2,
}));
const CONFETTI = Array.from({ length: 90 }, (_, i) => ({
  left: (i * 37 + 11) % 100,
  delay: (i % 15) * 0.09,
  duration: 2.2 + (i % 6) * 0.3,
  size: 6 + (i % 5) * 2,
  drift: ((i % 7) - 3) * 26,
  color: COLORS[i % COLORS.length]!,
}));

function splitDuration(ms: number) {
  const total = Math.max(0, Math.ceil(ms / 1000));
  return {
    jours: Math.floor(total / 86400),
    heures: Math.floor((total % 86400) / 3600),
    minutes: Math.floor((total % 3600) / 60),
    secondes: total % 60,
    total,
  };
}

export default function SantaReveal({
  revealAt,
  serverOffsetMs,
  isParticipant,
  mine,
  onDue,
  onOpened,
}: {
  revealAt: string;
  /** Heure du serveur moins heure du poste. */
  serverOffsetMs: number;
  isParticipant: boolean;
  mine: {
    receiverKey: string;
    receiverName: string;
    photoUrl: string | null;
    alreadyOpened: boolean;
    changed: boolean;
  } | null;
  /** Demande le résultat au serveur (rappelé tant qu'il n'est pas arrivé). */
  onDue: () => void;
  /** Reçoit le repère de la personne affichée. */
  onOpened: (seenReceiver: string) => void;
}) {
  const target = new Date(revealAt).getTime();
  const [remaining, setRemaining] = useState(() => target - (Date.now() + serverOffsetMs));
  const [phase, setPhase] = useState<GiftPhase>("closed");
  /** La personne a attendu devant le compte à rebours : le cadeau s'ouvrira tout seul. */
  const waited = useRef(remaining > 0);
  const callbacks = useRef({ onDue, onOpened });
  callbacks.current = { onDue, onOpened };
  const due = remaining <= 0;

  useEffect(() => {
    const tick = () => setRemaining(target - (Date.now() + serverOffsetMs));
    tick();
    const timer = window.setInterval(tick, 250);
    return () => window.clearInterval(timer);
  }, [target, serverOffsetMs]);

  const waitingResult = due && !mine && isParticipant;
  useEffect(() => {
    if (!waitingResult) return;
    callbacks.current.onDue();
    const timer = window.setInterval(() => callbacks.current.onDue(), 3000);
    return () => window.clearInterval(timer);
  }, [waitingResult]);

  const ready = Boolean(mine);
  useEffect(() => {
    if (ready && waited.current) {
      waited.current = false;
      setPhase("shaking");
    }
  }, [ready]);

  useEffect(() => {
    if (phase !== "shaking") return;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const timer = window.setTimeout(
      () => {
        setPhase("open");
        callbacks.current.onOpened(keyRef.current);
      },
      reduced ? 0 : GIFT_SHAKE_MS,
    );
    return () => window.clearTimeout(timer);
  }, [phase]);

  /** Le nom a changé pendant que la page était ouverte : on referme le cadeau. */
  const receiverKey = mine?.receiverKey ?? null;
  const [shownKey, setShownKey] = useState(receiverKey);
  if (receiverKey !== shownKey) {
    setShownKey(receiverKey);
    if (shownKey) setPhase("closed");
  }
  const keyRef = useRef("");
  keyRef.current = receiverKey ?? "";

  const parts = splitDuration(remaining);
  const lastSeconds = !due && parts.total <= 10;

  return (
    <section className="relative flex min-h-[24rem] flex-col justify-center overflow-hidden rounded-3xl bg-gradient-to-b from-red-800 via-red-900 to-slate-900 px-4 py-12 text-center text-white shadow-xl">
      <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden">
        {SNOW.map((f, i) => (
          <span
            key={i}
            className="santa-snow absolute top-[-5%] rounded-full bg-white/70"
            style={{
              left: `${f.left}%`,
              width: f.size,
              height: f.size,
              animationDelay: `${f.delay}s`,
              animationDuration: `${f.duration}s`,
            }}
          />
        ))}
        {phase === "open" &&
          CONFETTI.map((p, i) => (
            <span
              key={`c${i}`}
              className="santa-confetti absolute top-[-8%]"
              style={{
                left: `${p.left}%`,
                width: p.size,
                height: p.size * 1.6,
                background: p.color,
                animationDelay: `${p.delay}s`,
                animationDuration: `${p.duration}s`,
                ["--santa-drift" as string]: `${p.drift}px`,
              }}
            />
          ))}
      </div>

      <div className="relative">
        {!due ? (
          <>
            <p className="text-sm font-bold uppercase tracking-[0.25em] text-amber-200">Tirage dans</p>
            {lastSeconds ? (
              <p key={parts.total} className="santa-pop mt-4 text-8xl font-black tabular-nums" aria-live="assertive">
                {parts.total}
              </p>
            ) : (
              <div className="mt-4 flex justify-center gap-2 sm:gap-4" role="timer">
                {(["jours", "heures", "minutes", "secondes"] as const).map((unit) => (
                  <div key={unit} className="min-w-[4.2rem] rounded-2xl bg-white/10 px-2 py-3 sm:min-w-[6rem]">
                    <div className="text-3xl font-black tabular-nums sm:text-5xl">
                      {String(parts[unit]).padStart(2, "0")}
                    </div>
                    <div className="mt-1 text-[10px] font-bold uppercase text-white/70">{unit}</div>
                  </div>
                ))}
              </div>
            )}
            <div className="mt-4">
              <SantaGift phase="closed" head={null} />
            </div>
            <p className="mt-4 text-sm text-white/80">
              {isParticipant
                ? "Restez sur cette page : votre cadeau s'ouvrira tout seul à l'heure du tirage."
                : "Les participants découvriront leur nom à cette heure-là. Sans votre « oui », vous n'êtes pas dans le tirage."}
            </p>
          </>
        ) : !isParticipant ? (
          <>
            <p className="text-6xl" aria-hidden>
              🎅
            </p>
            <p className="mt-4 text-xl font-black">Le tirage a eu lieu.</p>
            <p className="mt-2 text-sm text-white/80">
              Vous ne participez pas : chaque personne ne voit que son propre résultat.
            </p>
          </>
        ) : !mine ? (
          <p className="py-10 text-lg font-bold" role="status">
            Le Père Noël tire les noms…
          </p>
        ) : (
          <>
            {phase === "open" && (
              <p className="text-sm font-bold uppercase tracking-[0.25em] text-amber-200">Vous offrez un cadeau à</p>
            )}
            <SantaGift phase={phase} head={{ name: mine.receiverName, photoUrl: mine.photoUrl }} />
            {phase === "open" ? (
              <div aria-live="polite">
                <p className="santa-pop santa-name mt-5 break-words text-4xl font-black sm:text-6xl">
                  {mine.receiverName}
                </p>
                <p className="mt-5 text-sm text-white/80">Chut… c&apos;est un secret. 🤫</p>
                <button
                  type="button"
                  onClick={() => setPhase("closed")}
                  className="mt-4 rounded-xl bg-white/15 px-5 py-2 text-sm font-bold hover:bg-white/25"
                >
                  Masquer
                </button>
              </div>
            ) : phase === "shaking" ? (
              <p className="mt-6 text-lg font-bold" role="status">
                Roulement de tambour…
              </p>
            ) : (
              <>
                <p className="mt-6 text-lg font-bold">
                  {mine.changed ? "Du changement : rouvrez votre cadeau" : "Le tirage a eu lieu !"}
                </p>
                <p className="mt-1 text-sm text-white/80">Vérifiez que personne ne regarde votre écran.</p>
                <button
                  type="button"
                  onClick={() => setPhase("shaking")}
                  className="mt-5 rounded-2xl bg-amber-300 px-8 py-4 text-base font-black text-red-900 shadow-lg hover:bg-amber-200"
                >
                  {mine.alreadyOpened ? "Revoir mon nom" : "Ouvrir mon cadeau"}
                </button>
              </>
            )}
          </>
        )}
      </div>

      <style>{`
        @keyframes santa-snow-fall {
          0% { transform: translate3d(0, 0, 0); }
          100% { transform: translate3d(30px, 640px, 0); }
        }
        @keyframes santa-confetti-fall {
          0% { opacity: 0; transform: translate3d(0, 0, 0) rotate(0deg); }
          10% { opacity: 1; }
          100% { opacity: 0; transform: translate3d(var(--santa-drift, 20px), 620px, 0) rotate(540deg); }
        }
        @keyframes santa-pop-in {
          0% { opacity: 0; transform: scale(0.3); }
          70% { opacity: 1; transform: scale(1.12); }
          100% { opacity: 1; transform: scale(1); }
        }
        .santa-snow { animation: santa-snow-fall linear infinite; }
        .santa-confetti { animation: santa-confetti-fall cubic-bezier(0.25, 0.7, 0.35, 1) both; }
        .santa-pop { animation: santa-pop-in 0.6s ease-out both; }
        .santa-name { animation-delay: 0.7s; }
        @media (prefers-reduced-motion: reduce) {
          .santa-snow, .santa-confetti { display: none; }
          .santa-pop { animation: none; }
        }
      `}</style>
    </section>
  );
}
