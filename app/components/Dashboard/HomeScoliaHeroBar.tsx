"use client";

import { useEffect, useRef, useState, type FormEvent } from "react";
import ScoliaAiMark from "@/app/components/ScoliaAiMark";
import { askScolia, openScolia } from "@/app/lib/brain-ai/scolia-ask";
import { SCOLIA_AI_NAME } from "@/app/lib/brain-ai/scolia-memory";

/**
 * Barre hero ScolIA — chat first sur l’accueil.
 * Envoi / micro / focus → ouvre la modale conversation.
 */
export default function HomeScoliaHeroBar() {
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
      } else {
        openScolia();
      }
    };
    recognition.onerror = () => flushSend();
    recognition.onend = () => flushSend();
    recognition.start();
  };

  const submit = (e?: FormEvent) => {
    e?.preventDefault();
    const text = value.trim();
    if (!text) {
      openScolia();
      return;
    }
    askScolia(text);
    setValue("");
  };

  return (
    <section className="relative overflow-hidden rounded-[2rem] bg-[var(--dash-ink)] p-1 shadow-[0_24px_60px_-28px_rgba(0,0,0,0.55)]">
      <div
        className="pointer-events-none absolute -right-16 -top-16 h-48 w-48 rounded-full bg-[color:var(--dash-lime)]/35 blur-3xl"
        aria-hidden
      />
      <div
        className="pointer-events-none absolute -bottom-20 left-10 h-40 w-40 rounded-full bg-[color:var(--dash-lime)]/20 blur-3xl"
        aria-hidden
      />

      <form
        onSubmit={submit}
        className="relative flex flex-col gap-3 rounded-[1.75rem] bg-[var(--dash-ink)] px-4 py-4 sm:px-6 sm:py-5"
      >
        <div className="flex items-center gap-3">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-[color:var(--dash-lime)] shadow-sm ring-1 ring-black/5">
            <div className="flex h-9 w-9 items-center justify-center overflow-hidden rounded-xl bg-[var(--dash-ink)]">
              <ScoliaAiMark size="sm" inverted fill />
            </div>
          </div>
          <div className="min-w-0">
            <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--dash-lime)]">
              Chat first
            </p>
            <h1 className="truncate text-xl font-semibold tracking-tight text-white sm:text-2xl">
              {SCOLIA_AI_NAME}
            </h1>
          </div>
          <button
            type="button"
            onClick={() => openScolia()}
            className="ml-auto hidden rounded-2xl border border-white/15 bg-white/5 px-3 py-2 text-xs font-semibold text-white/80 transition hover:bg-white/10 sm:inline-flex"
          >
            Ouvrir le chat
          </button>
        </div>

        <div className="flex items-end gap-2 rounded-[1.5rem] bg-white p-2 shadow-inner sm:p-2.5">
          <input
            type="text"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onFocus={() => {
              /* garde le focus dans la barre ; la modale s’ouvre à l’envoi */
            }}
            placeholder="Demandez n’importe quoi — absences, PAP, salles, sorties…"
            className="min-h-[3rem] min-w-0 flex-1 bg-transparent px-3 py-2 text-base font-medium text-[var(--dash-ink)] outline-none placeholder:text-neutral-400 sm:text-[17px]"
            aria-label={`Message ${SCOLIA_AI_NAME}`}
          />
          {speechSupported ? (
            <button
              type="button"
              onClick={startVoice}
              className={`inline-flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl text-lg transition ${
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
            type="submit"
            className="inline-flex h-12 shrink-0 items-center justify-center rounded-2xl bg-[var(--dash-ink)] px-5 text-sm font-semibold text-white transition hover:opacity-90"
          >
            {value.trim() ? "Envoyer" : "Parler"}
          </button>
        </div>
      </form>
    </section>
  );
}
