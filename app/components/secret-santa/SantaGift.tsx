"use client";

import { useState } from "react";
import { initialsOf } from "@/app/components/messaging/conversation/Avatar";

/**
 * Le cadeau animé : il flotte, se secoue de plus en plus fort, le couvercle saute et la tête de la
 * personne tirée (photo, sinon initiales) jaillit avec un bonnet de Noël.
 */

export type GiftPhase = "closed" | "shaking" | "open";

export const GIFT_SHAKE_MS = 3200;

function SantaHat() {
  return (
    <svg viewBox="0 0 140 100" className="santa-hat" aria-hidden>
      {/* Bonnet qui retombe sur la droite */}
      <path d="M18 76 C24 36 52 6 84 10 C110 13 124 34 124 58 C112 42 100 38 92 42 C100 52 106 64 108 76 Z" fill="#dc2626" />
      <path d="M34 70 C40 44 58 24 82 18 C62 32 50 50 48 72 Z" fill="#ef4444" />
      {/* Bord en fourrure */}
      <rect x="10" y="68" width="106" height="22" rx="11" fill="#ffffff" />
      <circle cx="124" cy="62" r="13" fill="#ffffff" />
    </svg>
  );
}

function Head({ name, photoUrl }: { name: string; photoUrl: string | null }) {
  const [broken, setBroken] = useState<string | null>(null);
  const showPhoto = photoUrl && broken !== photoUrl;
  return (
    <>
      <span className="santa-face">
        {showPhoto ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img src={photoUrl} alt="" onError={() => setBroken(photoUrl)} />
        ) : (
          <span className="santa-initials">{initialsOf(name)}</span>
        )}
      </span>
      <SantaHat />
    </>
  );
}

export default function SantaGift({
  phase,
  head,
}: {
  phase: GiftPhase;
  /** Présente dès la secousse (cachée dans la boîte) pour que la photo soit déjà chargée. */
  head: { name: string; photoUrl: string | null } | null;
}) {
  return (
    <div className={`santa-stage santa-stage-${phase}`} aria-hidden>
      {phase === "open" && <span className="santa-glow" />}
      {head && phase !== "closed" && (
        <div className="santa-head">
          <Head name={head.name} photoUrl={head.photoUrl} />
        </div>
      )}
      <div className="santa-gift">
        <div className="santa-lid">
          <span className="santa-bow santa-bow-left" />
          <span className="santa-bow santa-bow-right" />
          <span className="santa-knot" />
          <span className="santa-lid-box">
            <span className="santa-ribbon" />
          </span>
        </div>
        <div className="santa-box">
          <span className="santa-ribbon" />
        </div>
      </div>

      <style>{`
        .santa-stage { position: relative; width: 260px; height: 190px; margin: 0 auto; }
        .santa-stage-shaking, .santa-stage-open { height: 340px; }

        .santa-gift { position: absolute; left: 50%; bottom: 8px; width: 170px; height: 150px; margin-left: -85px; z-index: 2; transform-origin: 50% 100%; }
        .santa-box { position: absolute; left: 7px; right: 7px; bottom: 0; height: 108px; border-radius: 6px 6px 16px 16px; background: linear-gradient(135deg, #22c55e, #15803d 70%); box-shadow: inset 0 14px 0 rgba(0,0,0,0.16), 0 18px 24px rgba(0,0,0,0.35); overflow: hidden; }
        .santa-lid { position: absolute; left: 0; right: 0; bottom: 100px; height: 34px; z-index: 3; }
        .santa-lid-box { position: absolute; inset: 0; border-radius: 8px; background: linear-gradient(135deg, #4ade80, #16a34a 70%); box-shadow: 0 5px 8px rgba(0,0,0,0.25); overflow: hidden; }
        .santa-ribbon { position: absolute; top: 0; bottom: 0; left: 50%; width: 28px; margin-left: -14px; background: linear-gradient(90deg, #f59e0b, #fde68a 45%, #f59e0b); }
        .santa-bow { position: absolute; bottom: 30px; width: 46px; height: 30px; border: 9px solid #fbbf24; border-radius: 50% 50% 50% 50% / 60% 60% 40% 40%; }
        .santa-bow-left { left: 44px; transform: rotate(22deg); }
        .santa-bow-right { right: 44px; transform: rotate(-22deg); }
        .santa-knot { position: absolute; left: 50%; bottom: 29px; width: 24px; height: 18px; margin-left: -12px; border-radius: 9px; background: #f59e0b; z-index: 1; }

        /* La tête reste derrière la boîte : elle semble vraiment en sortir. */
        .santa-head { position: absolute; left: 50%; bottom: 98px; width: 168px; height: 168px; margin-left: -84px; z-index: 1; opacity: 0; transform: translateY(70px) scale(0.2); }
        .santa-face { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; overflow: hidden; border-radius: 9999px; border: 6px solid #ffffff; background: #fde68a; box-shadow: 0 10px 30px rgba(0,0,0,0.35); }
        .santa-face img { width: 100%; height: 100%; object-fit: cover; }
        .santa-initials { font-size: 60px; font-weight: 900; color: #7f1d1d; }
        .santa-hat { position: absolute; top: -54px; left: 14px; width: 154px; transform: rotate(9deg); filter: drop-shadow(0 4px 4px rgba(0,0,0,0.3)); }

        .santa-glow { position: absolute; left: 50%; bottom: 40px; width: 340px; height: 340px; margin-left: -170px; border-radius: 9999px; background: radial-gradient(circle, rgba(253,230,138,0.55), rgba(253,230,138,0) 65%); animation: santa-glow-in 0.9s ease-out both; }

        .santa-stage-closed .santa-gift { animation: santa-float-box 2.4s ease-in-out infinite; }
        .santa-stage-shaking .santa-gift { animation: santa-shake-box ${GIFT_SHAKE_MS}ms linear both; }
        .santa-stage-shaking .santa-lid { animation: santa-lid-rattle ${GIFT_SHAKE_MS}ms linear both; }
        .santa-stage-open .santa-gift { animation: santa-recoil 0.55s ease-out both; }
        .santa-stage-open .santa-lid { animation: santa-lid-fly 0.9s cubic-bezier(0.2, 0.7, 0.3, 1) both; }
        .santa-stage-open .santa-head { animation: santa-head-pop 0.9s cubic-bezier(0.22, 1, 0.36, 1) 0.12s both, santa-head-bob 2.6s ease-in-out 1.1s infinite; }

        @keyframes santa-float-box { 0%, 100% { transform: translateY(0); } 50% { transform: translateY(-10px); } }
        @keyframes santa-shake-box {
          0% { transform: translate(0, 0) rotate(0deg); }
          6% { transform: translate(-3px, 0) rotate(-3deg); }
          11% { transform: translate(3px, 0) rotate(3deg); }
          16% { transform: translate(-3px, 0) rotate(-3deg); }
          21% { transform: translate(3px, 0) rotate(2deg); }
          28% { transform: translate(0, 0) rotate(0deg); }
          34% { transform: translate(-7px, -2px) rotate(-7deg); }
          39% { transform: translate(7px, 0) rotate(7deg); }
          44% { transform: translate(-8px, -3px) rotate(-8deg); }
          49% { transform: translate(8px, 0) rotate(7deg); }
          56% { transform: translate(0, 0) rotate(0deg) scale(1); }
          61% { transform: translate(-11px, -8px) rotate(-11deg) scale(1.03); }
          65% { transform: translate(11px, 0) rotate(11deg) scale(1.05); }
          69% { transform: translate(-13px, -14px) rotate(-13deg) scale(1.07); }
          73% { transform: translate(13px, 0) rotate(12deg) scale(1.08); }
          77% { transform: translate(-14px, -18px) rotate(-14deg) scale(1.1); }
          81% { transform: translate(14px, 0) rotate(14deg) scale(1.1); }
          85% { transform: translate(-14px, -20px) rotate(-13deg) scale(1.12); }
          89% { transform: translate(12px, 0) rotate(12deg) scale(1.12); }
          /* Il se tasse juste avant d'éclater. */
          100% { transform: translate(0, 0) rotate(0deg) scale(1.2, 0.82); }
        }
        @keyframes santa-lid-rattle {
          0%, 30%, 56% { transform: translateY(0) rotate(0deg); }
          40% { transform: translateY(-5px) rotate(-3deg); }
          48% { transform: translateY(-2px) rotate(3deg); }
          66% { transform: translateY(-10px) rotate(-6deg); }
          74% { transform: translateY(-3px) rotate(5deg); }
          82% { transform: translateY(-16px) rotate(-8deg); }
          90% { transform: translateY(-6px) rotate(6deg); }
          100% { transform: translateY(0) rotate(0deg); }
        }
        @keyframes santa-recoil {
          0% { transform: scale(1.2, 0.82); }
          35% { transform: scale(0.9, 1.14); }
          65% { transform: scale(1.05, 0.96); }
          100% { transform: scale(1, 1); }
        }
        @keyframes santa-lid-fly {
          0% { transform: translate(0, 0) rotate(0deg); }
          55% { transform: translate(-80px, -190px) rotate(-150deg); }
          100% { transform: translate(-150px, 62px) rotate(-338deg); }
        }
        @keyframes santa-head-pop {
          0% { opacity: 0; transform: translateY(70px) scale(0.2) rotate(-25deg); }
          15% { opacity: 1; }
          55% { opacity: 1; transform: translateY(-34px) scale(1.12) rotate(9deg); }
          75% { opacity: 1; transform: translateY(10px) scale(0.96) rotate(-5deg); }
          100% { opacity: 1; transform: translateY(0) scale(1) rotate(0deg); }
        }
        @keyframes santa-head-bob { 0%, 100% { transform: translateY(0) rotate(0deg); } 25% { transform: translateY(-6px) rotate(-4deg); } 75% { transform: translateY(-6px) rotate(4deg); } }
        @keyframes santa-glow-in { 0% { opacity: 0; transform: scale(0.2); } 100% { opacity: 1; transform: scale(1); } }

        /* Sans animation : le résultat s'affiche directement. */
        .santa-stage-open .santa-head { opacity: 1; transform: none; }
        .santa-stage-open .santa-lid { transform: translate(-150px, 62px) rotate(-338deg); }
        @media (prefers-reduced-motion: reduce) {
          .santa-stage .santa-gift, .santa-stage .santa-lid, .santa-stage .santa-head, .santa-glow { animation: none !important; }
        }
      `}</style>
    </div>
  );
}
