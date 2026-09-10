"use client";

import { useEffect, useRef } from "react";
import {
  IconCamOff,
  IconMic,
  IconMicOff,
  IconPhoneOff,
  IconPictureInPicture,
  IconMaximize,
  IconVideoCall,
} from "./MessagingIcons";
import { useMessagingCall } from "./MessagingCallProvider";

export function CallVideoTile({
  stream,
  muted,
  label,
  mirror,
  className = "",
}: {
  stream: MediaStream | null;
  muted?: boolean;
  label: string;
  mirror?: boolean;
  className?: string;
}) {
  const ref = useRef<HTMLVideoElement | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.srcObject = stream;
  }, [stream]);

  return (
    <div className={`relative min-h-0 overflow-hidden rounded-xl bg-slate-900 ${className}`}>
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

type StageProps = {
  compact?: boolean;
  className?: string;
};

/** Grille vidéo + contrôles (micro / cam / PiP / split / raccrocher). */
export default function MessagingCallStage({ compact = false, className = "" }: StageProps) {
  const { call, hangUp, toggleMute, toggleCam, setViewMode } = useMessagingCall();
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
        ? compact
          ? "grid-cols-2"
          : "grid-cols-1 sm:grid-cols-2"
        : tiles.length <= 4
          ? "grid-cols-2"
          : "grid-cols-2 lg:grid-cols-3";

  const status =
    call.phase === "outgoing"
      ? "Appel…"
      : call.phase === "connecting"
        ? "Connexion…"
        : `${1 + call.remotes.length} en ligne`;

  return (
    <div className={`flex min-h-0 flex-col bg-slate-950 text-white ${className}`}>
      <header className="flex shrink-0 items-center justify-between gap-2 border-b border-white/10 px-3 py-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold">{call.title}</p>
          <p className="text-[11px] text-white/55">{status}</p>
        </div>
        <div className="flex items-center gap-1">
          {call.viewMode === "split" ? (
            <button
              type="button"
              title="Mode PiP"
              className="rounded-full p-2 text-white/75 hover:bg-white/10 hover:text-white"
              onClick={() => setViewMode("pip")}
            >
              <IconPictureInPicture className="h-4 w-4" />
            </button>
          ) : (
            <button
              type="button"
              title="Revenir à la messagerie + vidéo"
              className="rounded-full p-2 text-white/75 hover:bg-white/10 hover:text-white"
              onClick={() => setViewMode("split")}
            >
              <IconMaximize className="h-4 w-4" />
            </button>
          )}
        </div>
      </header>

      <div className={`grid min-h-0 flex-1 gap-2 overflow-hidden p-2 ${gridClass}`}>
        {tiles.map((t) => (
          <CallVideoTile
            key={t.key}
            stream={t.stream}
            label={t.label}
            muted={t.muted}
            mirror={t.mirror}
            className={compact ? "min-h-[4.5rem]" : "min-h-[9rem]"}
          />
        ))}
      </div>

      {call.error ? (
        <p className="shrink-0 px-3 pb-1 text-center text-xs text-red-300">{call.error}</p>
      ) : null}

      <div className="flex shrink-0 items-center justify-center gap-3 border-t border-white/10 px-3 py-3">
        <button
          type="button"
          title={call.muted ? "Réactiver le micro" : "Couper le micro"}
          className={`rounded-full p-2.5 ${call.muted ? "bg-red-600" : "bg-white/15 hover:bg-white/25"}`}
          onClick={toggleMute}
        >
          {call.muted ? <IconMicOff className="h-4 w-4" /> : <IconMic className="h-4 w-4" />}
        </button>
        <button
          type="button"
          title={call.camOff ? "Allumer la caméra" : "Couper la caméra"}
          className={`rounded-full p-2.5 ${call.camOff ? "bg-red-600" : "bg-white/15 hover:bg-white/25"}`}
          onClick={toggleCam}
        >
          {call.camOff ? (
            <IconCamOff className="h-4 w-4" />
          ) : (
            <IconVideoCall className="h-4 w-4" />
          )}
        </button>
        <button
          type="button"
          title="Raccrocher"
          className="rounded-full bg-red-600 p-2.5 hover:bg-red-500"
          onClick={() => void hangUp()}
        >
          <IconPhoneOff className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
