"use client";

import { useEffect, useRef, useState } from "react";
import { IconVideoCall } from "./MessagingIcons";
import { useMessagingCall } from "./MessagingCallProvider";
import MessagingCallStage from "./MessagingCallStage";

/**
 * - Sonnerie entrante (modal)
 * - Mode split flottant (panneau gauche) pour l’overlay global
 * - Mode PiP déplaçable
 * Sur /messagerie, le mode split est rendu dans la page (voir MessageriePageClient).
 */
export default function MessagingCallOverlay({
  embedSplitInPage = false,
}: {
  /** Si true, le split est géré par la page messagerie — ici seulement PiP + sonnerie. */
  embedSplitInPage?: boolean;
}) {
  const { call, incoming, acceptIncoming, rejectIncoming, setViewMode } = useMessagingCall();
  const [pipPos, setPipPos] = useState({ x: 24, y: 96 });
  const dragRef = useRef<{ ox: number; oy: number; sx: number; sy: number } | null>(null);

  useEffect(() => {
    if (!call || call.viewMode !== "pip") return;
    const onMove = (ev: PointerEvent) => {
      const d = dragRef.current;
      if (!d) return;
      setPipPos({
        x: Math.max(8, Math.min(window.innerWidth - 300, d.sx + (ev.clientX - d.ox))),
        y: Math.max(8, Math.min(window.innerHeight - 260, d.sy + (ev.clientY - d.oy))),
      });
    };
    const onUp = () => {
      dragRef.current = null;
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
  }, [call]);

  if (incoming && !call) {
    return (
      <div className="fixed inset-0 z-[160] flex items-center justify-center bg-black/50 p-4">
        <div className="w-full max-w-sm overflow-hidden rounded-2xl bg-white shadow-2xl">
          <div className="bg-gradient-to-br from-sky-600 to-indigo-700 px-5 py-6 text-center text-white">
            <IconVideoCall className="mx-auto mb-3 h-10 w-10" />
            <p className="text-lg font-semibold">Appel vidéo entrant</p>
            <p className="mt-1 text-sm text-sky-100">{incoming.fromName}</p>
            <p className="mt-0.5 text-xs text-sky-100/80">{incoming.title}</p>
          </div>
          <div className="flex gap-3 p-4">
            <button
              type="button"
              className="flex-1 rounded-full bg-slate-100 py-3 text-sm font-semibold text-slate-700 hover:bg-slate-200"
              onClick={() => void rejectIncoming()}
            >
              Refuser
            </button>
            <button
              type="button"
              className="flex-1 rounded-full bg-emerald-600 py-3 text-sm font-semibold text-white hover:bg-emerald-700"
              onClick={() => void acceptIncoming()}
            >
              Accepter
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (!call || call.phase === "idle") return null;

  if (call.viewMode === "pip") {
    return (
      <div
        className="fixed z-[158] w-[280px] overflow-hidden rounded-2xl shadow-2xl ring-1 ring-white/20"
        style={{ left: pipPos.x, top: pipPos.y }}
      >
        <div
          className="cursor-grab bg-slate-900/90 px-2 py-1 text-[10px] font-medium text-white/80 active:cursor-grabbing"
          onPointerDown={(e) => {
            dragRef.current = {
              ox: e.clientX,
              oy: e.clientY,
              sx: pipPos.x,
              sy: pipPos.y,
            };
          }}
        >
          Visio · glisser pour déplacer · double-clic = agrandir
        </div>
        <div
          className="h-[200px]"
          onDoubleClick={() => setViewMode("split")}
        >
          <MessagingCallStage compact className="h-full" />
        </div>
      </div>
    );
  }

  // Split géré par la page messagerie
  if (embedSplitInPage) return null;

  // Split flottant (dashboard / overlay) : vidéo à gauche, messagerie reste utilisable à droite
  return (
    <div className="pointer-events-none fixed inset-y-3 left-3 z-[155] flex w-[min(52vw,640px)] max-w-[calc(100vw-2rem)] flex-col sm:inset-y-4 sm:left-4">
      <div className="pointer-events-auto flex min-h-0 flex-1 overflow-hidden rounded-2xl shadow-2xl ring-1 ring-white/15">
        <MessagingCallStage className="h-full w-full" />
      </div>
      <p className="pointer-events-none mt-1 text-center text-[10px] text-slate-500">
        Continuez à chatter à droite · PiP via l’icône en haut de la visio
      </p>
    </div>
  );
}
