"use client";

import { useEffect, useRef } from "react";
import {
  IconCamOff,
  IconMic,
  IconMicOff,
  IconPhoneOff,
  IconVideoCall,
  IconX,
} from "./MessagingIcons";
import { useMessagingCall } from "./MessagingCallProvider";

function VideoTile({
  stream,
  muted,
  label,
  mirror,
}: {
  stream: MediaStream | null;
  muted?: boolean;
  label: string;
  mirror?: boolean;
}) {
  const ref = useRef<HTMLVideoElement | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.srcObject = stream;
  }, [stream]);

  return (
    <div className="relative overflow-hidden rounded-xl bg-slate-900">
      <video
        ref={ref}
        autoPlay
        playsInline
        muted={muted}
        className={`h-full w-full object-cover ${mirror ? "scale-x-[-1]" : ""}`}
      />
      <span className="absolute bottom-2 left-2 rounded-full bg-black/55 px-2 py-0.5 text-[11px] font-medium text-white">
        {label}
      </span>
    </div>
  );
}

export default function MessagingCallOverlay() {
  const {
    call,
    incoming,
    acceptIncoming,
    rejectIncoming,
    hangUp,
    toggleMute,
    toggleCam,
  } = useMessagingCall();

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

  const tiles = [
    {
      key: "local",
      stream: call.localStream,
      label: call.camOff ? "Vous (cam off)" : "Vous",
      muted: true,
      mirror: true,
    },
    ...call.remotes.map((r) => ({
      key: r.userId,
      stream: r.stream,
      label: r.name,
      muted: false,
      mirror: false,
    })),
  ];

  const gridClass =
    tiles.length <= 1
      ? "grid-cols-1"
      : tiles.length === 2
        ? "grid-cols-1 sm:grid-cols-2"
        : tiles.length <= 4
          ? "grid-cols-2"
          : "grid-cols-2 lg:grid-cols-3";

  return (
    <div className="fixed inset-0 z-[160] flex flex-col bg-slate-950/95 text-white">
      <header className="flex items-center justify-between border-b border-white/10 px-4 py-3">
        <div>
          <p className="text-sm font-semibold">{call.title}</p>
          <p className="text-xs text-white/60">
            {call.phase === "outgoing"
              ? "Appel en cours…"
              : call.phase === "connecting"
                ? "Connexion…"
                : `${1 + call.remotes.length} participant${1 + call.remotes.length > 1 ? "s" : ""}`}
          </p>
        </div>
        <button
          type="button"
          className="rounded-full p-2 text-white/70 hover:bg-white/10 hover:text-white"
          onClick={() => void hangUp()}
          aria-label="Fermer"
        >
          <IconX className="h-5 w-5" />
        </button>
      </header>

      <div className={`grid flex-1 gap-2 overflow-auto p-3 ${gridClass}`}>
        {tiles.map((t) => (
          <VideoTile
            key={t.key}
            stream={t.stream}
            label={t.label}
            muted={t.muted}
            mirror={t.mirror}
          />
        ))}
      </div>

      {call.error ? (
        <p className="px-4 pb-2 text-center text-sm text-red-300">{call.error}</p>
      ) : null}

      <div className="flex items-center justify-center gap-4 border-t border-white/10 px-4 py-4">
        <button
          type="button"
          title={call.muted ? "Réactiver le micro" : "Couper le micro"}
          className={`rounded-full p-3 ${call.muted ? "bg-red-600" : "bg-white/15 hover:bg-white/25"}`}
          onClick={toggleMute}
        >
          {call.muted ? <IconMicOff className="h-5 w-5" /> : <IconMic className="h-5 w-5" />}
        </button>
        <button
          type="button"
          title={call.camOff ? "Allumer la caméra" : "Couper la caméra"}
          className={`rounded-full p-3 ${call.camOff ? "bg-red-600" : "bg-white/15 hover:bg-white/25"}`}
          onClick={toggleCam}
        >
          {call.camOff ? (
            <IconCamOff className="h-5 w-5" />
          ) : (
            <IconVideoCall className="h-5 w-5" />
          )}
        </button>
        <button
          type="button"
          title="Raccrocher"
          className="rounded-full bg-red-600 p-3 hover:bg-red-500"
          onClick={() => void hangUp()}
        >
          <IconPhoneOff className="h-5 w-5" />
        </button>
      </div>
    </div>
  );
}
