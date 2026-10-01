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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const recognitionRef = useRef<any>(null);

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
    setListening(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    recognition.onresult = (event: any) => {
      let finalText = "";
      for (let i = event.resultIndex; i < event.results.length; i += 1) {
        const t = event.results[i][0]?.transcript || "";
        if (event.results[i].isFinal) finalText += t;
        else setValue(t.trim());
      }
      if (finalText.trim()) {
        setValue(finalText.trim());
        askScolia(finalText.trim());
        setValue("");
      }
    };
    recognition.onerror = () => stopVoice();
    recognition.onend = () => stopVoice();
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
      className={`flex items-center gap-2 rounded-2xl border border-white/70 bg-white/80 px-2.5 py-2 shadow-sm backdrop-blur-xl ${className}`}
    >
      <ScoliaAiMark className="h-7 w-7 shrink-0" />
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
        className="min-w-0 flex-1 bg-transparent text-sm text-slate-800 outline-none placeholder:text-slate-400"
        aria-label={`Message ${SCOLIA_AI_NAME}`}
      />
      {speechSupported ? (
        <button
          type="button"
          onClick={startVoice}
          className={`inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full text-sm ${
            listening
              ? "bg-rose-500 text-white animate-pulse"
              : "bg-slate-100 text-slate-700 hover:bg-slate-200"
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
        className="shrink-0 rounded-full bg-slate-900 px-3 py-1.5 text-xs font-semibold text-white hover:bg-black"
      >
        Envoyer
      </button>
      <Link
        href="/dashboard"
        className="hidden shrink-0 rounded-full border border-slate-200 px-2.5 py-1.5 text-[11px] font-medium text-slate-600 hover:bg-slate-50 sm:inline"
        title="Hub ScolIA"
      >
        Hub
      </Link>
    </div>
  );
}
