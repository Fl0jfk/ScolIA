"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import ScoliaAiMark from "@/app/components/ScoliaAiMark";
import { askScolia, openScolia } from "@/app/lib/brain-ai/scolia-ask";
import { SCOLIA_AI_NAME } from "@/app/lib/brain-ai/scolia-memory";

/**
 * Champ ScolIA compact pour les hubs / liveboards — ne remplace pas le hub d’accueil.
 */
export default function ScoliaCompactBar({ className = "" }: { className?: string }) {
  const [value, setValue] = useState("");
  const [listening, setListening] = useState(false);
  const [speechSupported, setSpeechSupported] = useState(false);
  const voiceFinalRef = useRef("");
  const valueRef = useRef("");
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const recognitionRef = useRef<any>(null);

  useEffect(() => {
    valueRef.current = value;
  }, [value]);

  useEffect(() => {
    setSpeechSupported(
      typeof window !== "undefined" &&
        ("webkitSpeechRecognition" in window || "SpeechRecognition" in window),
    );
  }, []);

  const stopVoice = () => {
    try {
      recognitionRef.current?.stop();
    } catch {
      /* ignore */
    }
    recognitionRef.current = null;
    setListening(false);
  };

  const startVoice = () => {
    if (!speechSupported) return;
    if (listening) {
      stopVoice();
      return;
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const Ctor = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (!Ctor) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const recognition = new (Ctor as any)();
    recognition.lang = "fr-FR";
    recognition.interimResults = true;
    recognition.continuous = false;
    recognitionRef.current = recognition;
    voiceFinalRef.current = "";
    let flushed = false;
    setListening(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    recognition.onresult = (event: any) => {
      let interim = "";
      let finalText = "";
      for (let i = event.resultIndex; i < event.results.length; i += 1) {
        const t = event.results[i][0]?.transcript || "";
        if (event.results[i].isFinal) finalText += t;
        else interim += t;
      }
      if (interim.trim()) setValue(interim.trim());
      if (finalText.trim()) {
        const chunk = finalText.trim();
        voiceFinalRef.current = voiceFinalRef.current
          ? `${voiceFinalRef.current} ${chunk}`
          : chunk;
        setValue(voiceFinalRef.current);
      }
    };
    const flushSend = () => {
      if (flushed) return;
      flushed = true;
      const text = voiceFinalRef.current.trim() || valueRef.current.trim();
      voiceFinalRef.current = "";
      recognitionRef.current = null;
      setListening(false);
      if (text) {
        setValue("");
        askScolia(text);
      }
    };
    recognition.onerror = () => flushSend();
    recognition.onend = () => flushSend();
    recognition.start();
  };

  const submit = () => {
    const text = value.trim();
    if (!text) {
      openScolia();
      return;
    }
    askScolia(text);
    setValue("");
  };

  return (
    <div
      className={`flex items-center gap-2 rounded-[1.5rem] bg-white p-2 ring-1 ring-black/6 shadow-sm ${className}`}
    >
      <span className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-[var(--dash-ink)]">
        <ScoliaAiMark size="sm" inverted fill />
      </span>
      <input
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            submit();
          }
        }}
        placeholder={`Demander à ${SCOLIA_AI_NAME}…`}
        className="min-w-0 flex-1 bg-transparent px-1 text-sm font-medium text-[var(--dash-ink)] outline-none placeholder:text-neutral-400"
        aria-label={`Message ${SCOLIA_AI_NAME}`}
      />
      {speechSupported ? (
        <button
          type="button"
          onClick={startVoice}
          className={`inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-2xl text-sm transition ${
            listening
              ? "bg-rose-500 text-white animate-pulse"
              : "bg-[#f0f1ee] text-[var(--dash-ink)] hover:bg-[color:var(--dash-lime)]"
          }`}
          aria-label={listening ? "Arrêter le micro" : "Dicter"}
          title="Vocal"
        >
          🎤
        </button>
      ) : null}
      <button
        type="button"
        onClick={submit}
        className="shrink-0 rounded-2xl bg-[var(--dash-ink)] px-3 py-2 text-xs font-semibold text-white transition hover:opacity-90"
      >
        Envoyer
      </button>
      <Link
        href="/dashboard"
        className="hidden shrink-0 rounded-2xl border border-black/8 px-2.5 py-2 text-[11px] font-semibold text-neutral-600 transition hover:bg-[#f0f1ee] sm:inline"
        title="Accueil"
      >
        Accueil
      </Link>
    </div>
  );
}
