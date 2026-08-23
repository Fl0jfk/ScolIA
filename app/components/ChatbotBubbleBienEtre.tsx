"use client";

import { Fragment, useCallback, useEffect, useRef, useState } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";

type BubbleMessage = {
  role: "user" | "assistant";
  content: string;
};

const CONSENT_KEY = "bien_etre_consent_v1";

function renderMessageContent(content: string) {
  const lines = content.split("\n");
  return lines.map((line, lineIndex) => (
    <Fragment key={`line_${lineIndex}`}>
      {line}
      {lineIndex < lines.length - 1 && <br />}
    </Fragment>
  ));
}

export default function ChatbotBubbleBienEtre() {
  const { isLoaded, isSignedIn } = useSessionUser();
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [listening, setListening] = useState(false);
  const [input, setInput] = useState("");
  const [mounted, setMounted] = useState(false);
  const [speechSupported, setSpeechSupported] = useState(false);
  const [messages, setMessages] = useState<BubbleMessage[]>([]);
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [welcome, setWelcome] = useState("");
  const [suggestSignalement, setSuggestSignalement] = useState(false);
  const [consentOk, setConsentOk] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [prenom, setPrenom] = useState("");
  const [classe, setClasse] = useState("");
  const [complement, setComplement] = useState("");
  const [signaling, setSignaling] = useState(false);
  const [banner, setBanner] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const messagesRef = useRef<HTMLDivElement | null>(null);
  const scrollYRef = useRef(0);

  useEffect(() => {
    setMounted(true);
    setSpeechSupported("webkitSpeechRecognition" in window || "SpeechRecognition" in window);
    try {
      setConsentOk(sessionStorage.getItem(CONSENT_KEY) === "1");
    } catch {
      setConsentOk(false);
    }
  }, []);

  const loadSession = useCallback(async () => {
    const res = await fetch("/api/bien-etre/chat", { cache: "no-store" });
    if (!res.ok) return;
    const data = await res.json();
    setEnabled(data.enabled === true);
    setWelcome(String(data.welcomeMessage || ""));
    setMessages(Array.isArray(data.messages) ? data.messages : []);
    setSuggestSignalement(data.suggestSignalement === true);
  }, []);

  useEffect(() => {
    if (isLoaded && isSignedIn && open) loadSession();
  }, [isLoaded, isSignedIn, open, loadSession]);

  useEffect(() => {
    if (!open) return;
    const body = document.body;
    const html = document.documentElement;
    const previousBodyOverflow = body.style.overflow;
    const previousBodyPosition = body.style.position;
    const previousBodyTop = body.style.top;
    const previousBodyLeft = body.style.left;
    const previousBodyRight = body.style.right;
    const previousBodyWidth = body.style.width;
    const previousHtmlOverscroll = html.style.overscrollBehavior;
    scrollYRef.current = window.scrollY;
    body.style.overflow = "hidden";
    body.style.position = "fixed";
    body.style.top = `-${scrollYRef.current}px`;
    body.style.left = "0";
    body.style.right = "0";
    body.style.width = "100%";
    html.style.overscrollBehavior = "none";
    return () => {
      body.style.overflow = previousBodyOverflow;
      body.style.position = previousBodyPosition;
      body.style.top = previousBodyTop;
      body.style.left = previousBodyLeft;
      body.style.right = previousBodyRight;
      body.style.width = previousBodyWidth;
      html.style.overscrollBehavior = previousHtmlOverscroll;
      window.scrollTo(0, scrollYRef.current);
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const el = messagesRef.current;
    if (!el) return;
    el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }, [messages, loading, open, banner, consentOk]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (panelRef.current?.contains(target)) return;
      if (buttonRef.current?.contains(target)) return;
      setOpen(false);
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [open]);

  const acceptConsent = () => {
    try {
      sessionStorage.setItem(CONSENT_KEY, "1");
    } catch {
      /* ignore */
    }
    setConsentOk(true);
  };

  const startVoiceInput = () => {
    if (!speechSupported || listening || loading) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const SpeechRecognitionCtor = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (!SpeechRecognitionCtor) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const recognition = new (SpeechRecognitionCtor as any)();
    recognition.lang = "fr-FR";
    recognition.interimResults = false;
    recognition.maxAlternatives = 1;
    setListening(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    recognition.onresult = (event: any) => {
      const transcript = event?.results?.[0]?.[0]?.transcript?.trim();
      if (transcript) setInput((prev) => (prev ? `${prev} ${transcript}` : transcript));
    };
    recognition.onerror = () => setListening(false);
    recognition.onend = () => setListening(false);
    recognition.start();
  };

  const send = async () => {
    const message = input.trim();
    if (!message || loading || !enabled || !consentOk) return;
    setInput("");
    setMessages((prev) => [...prev, { role: "user", content: message }]);
    setLoading(true);
    setBanner(null);
    try {
      const res = await fetch("/api/bien-etre/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message }),
      });
      const data = await res.json();
      if (!res.ok) {
        setBanner(data.error || "Erreur");
        return;
      }
      setMessages(data.messages || []);
      setSuggestSignalement(data.suggestSignalement === true);
    } catch {
      setBanner("Erreur réseau, réessaie dans un instant.");
    } finally {
      setLoading(false);
    }
  };

  const submitSignalement = async () => {
    if (signaling) return;
    setSignaling(true);
    setBanner(null);
    try {
      const res = await fetch("/api/bien-etre/signaler", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ prenom, classe, complement }),
      });
      const data = await res.json();
      if (!res.ok) {
        setBanner(data.error || "Erreur");
        return;
      }
      setModalOpen(false);
      setMessages([]);
      setSuggestSignalement(false);
      setPrenom("");
      setClasse("");
      setComplement("");
      setBanner(data.message || "Signalement envoyé au psychologue.");
    } catch {
      setBanner("Erreur réseau.");
    } finally {
      setSignaling(false);
    }
  };

  if (!isLoaded || !isSignedIn) return null;

  return (
    <>
      {open ? (
        <div className="fixed inset-0 z-[120] bg-violet-950/25 md:hidden" onClick={() => setOpen(false)} aria-hidden="true" />
      ) : null}
      {open && mounted ? (
      <div
        ref={panelRef}
        className="fixed inset-0 z-[120] h-[100dvh] overflow-hidden rounded-none border-0 bg-white/22 backdrop-blur-3xl md:inset-auto md:right-4 md:bottom-20 md:h-[570px] md:w-[min(92vw,390px)] md:rounded-[30px] md:border md:border-violet-200/60 md:shadow-[0_30px_80px_rgba(88,28,135,0.25)]"
      >
        <div className="relative h-full flex flex-col">
          <div className="px-4 py-3 pt-[max(12px,env(safe-area-inset-top))] bg-gradient-to-r from-violet-900 via-violet-800 to-violet-900 text-white flex items-center justify-between border-b border-violet-400/30">
            <div>
              <p className="text-[10px] font-bold uppercase tracking-widest text-violet-200">Bien-être</p>
              <p className="text-sm font-semibold">Bot d&apos;écoute</p>
            </div>
            <button type="button" onClick={() => setOpen(false)} className="text-xs opacity-80 hover:opacity-100">
              Fermer
            </button>
          </div>

          {!consentOk ? (
            <div className="flex-1 overflow-y-auto p-4 space-y-3 text-sm text-slate-700">
              <p className="font-bold text-violet-900">Avant de commencer</p>
              <p>Cette conversation <strong>n&apos;est pas enregistrée</strong> (elle disparaît si tu fermes ou recharges).</p>
              <p>Un <strong>signalement</strong> au psychologue est volontaire et seul ce choix conserve ton prénom.</p>
              <p className="text-xs">
                Urgence : <a href="tel:112" className="font-bold text-red-600">112</a> ·{" "}
                <a href="tel:3018" className="font-bold">3018</a> · <a href="tel:116" className="font-bold">116</a>
              </p>
              <button type="button" onClick={acceptConsent} className="w-full rounded-xl bg-violet-600 text-white font-bold py-2.5">
                J&apos;ai compris
              </button>
            </div>
          ) : enabled === false ? (
            <div className="flex-1 p-4 text-sm text-slate-600">Le bot bien-être n&apos;est pas activé. Parle à un adulte de confiance.</div>
          ) : (
            <>
              <div className="shrink-0 bg-amber-50 border-b border-amber-100 px-3 py-1.5 text-[10px] text-amber-900 text-center">
                Non enregistré — disparaît si tu quittes la page
              </div>
              {banner ? <div className="shrink-0 bg-violet-100 text-violet-900 text-xs px-3 py-2 text-center">{banner}</div> : null}
              <div ref={messagesRef} className="relative flex-1 min-h-0 overflow-y-auto p-3 space-y-2">
                {messages.length === 0 && welcome ? (
                  <div className="rounded-xl border border-violet-200/70 bg-violet-50/80 p-3 text-sm text-slate-700">{welcome}</div>
                ) : null}
                {messages.map((m, i) => (
                  <div
                    key={i}
                    className={`rounded-xl px-3 py-2 text-sm whitespace-pre-wrap ${
                      m.role === "user"
                        ? "bg-violet-600 text-white ml-8 border border-violet-500/50"
                        : "bg-white/70 text-slate-800 mr-8 border border-violet-100"
                    }`}
                  >
                    {renderMessageContent(m.content)}
                  </div>
                ))}
                {loading ? <p className="text-xs text-violet-400 animate-pulse px-1">Réflexion…</p> : null}
              </div>
              <div className="p-3 pb-[max(12px,env(safe-area-inset-bottom))] border-t border-violet-100 bg-white/30 space-y-2">
                <div className="flex justify-center gap-3 text-[10px] font-bold">
                  <a href="tel:116" className="text-teal-700">116</a>
                  <a href="tel:3018" className="text-teal-700">3018</a>
                  <a href="tel:112" className="text-red-600">112</a>
                </div>
                {(suggestSignalement || messages.length >= 2) && (
                  <button
                    type="button"
                    onClick={() => setModalOpen(true)}
                    className="w-full rounded-xl border-2 border-violet-300 bg-violet-50 text-violet-900 font-bold py-1.5 text-xs"
                  >
                    Signaler au psychologue
                  </button>
                )}
                <div className="flex gap-2">
                  <input
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && send()}
                    placeholder="Écris ici…"
                    disabled={loading}
                    className="flex-1 rounded-xl border border-violet-200 bg-white/80 px-3 py-2 text-sm"
                  />
                  <button type="button" onClick={startVoiceInput} disabled={!speechSupported || loading || listening} className="rounded-xl border px-2 text-sm">
                    {listening ? "…" : "🎤"}
                  </button>
                  <button type="button" onClick={send} disabled={loading} className="rounded-xl bg-violet-700 text-white px-3 py-2 text-sm font-bold disabled:opacity-50">
                    {loading ? "…" : "→"}
                  </button>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
      ) : null}

      {modalOpen ? (
        <div className="fixed inset-0 z-[130] flex items-end sm:items-center justify-center bg-black/40 p-4 pointer-events-auto">
          <div className="w-full max-w-md rounded-3xl bg-white p-5 shadow-2xl space-y-3 pointer-events-auto">
            <h2 className="text-lg font-black">Signalement</h2>
            <label className="block text-sm font-semibold">
              Prénom *
              <input value={prenom} onChange={(e) => setPrenom(e.target.value)} className="mt-1 w-full rounded-xl border px-3 py-2" />
            </label>
            <label className="block text-sm font-semibold">
              Classe (optionnel)
              <input value={classe} onChange={(e) => setClasse(e.target.value)} className="mt-1 w-full rounded-xl border px-3 py-2" />
            </label>
            <label className="block text-sm font-semibold">
              Message (optionnel)
              <textarea value={complement} onChange={(e) => setComplement(e.target.value)} rows={2} className="mt-1 w-full rounded-xl border px-3 py-2" />
            </label>
            <div className="flex gap-2">
              <button type="button" onClick={() => setModalOpen(false)} className="flex-1 rounded-xl border py-2 font-semibold">
                Annuler
              </button>
              <button
                type="button"
                onClick={submitSignalement}
                disabled={signaling || prenom.trim().length < 2}
                className="flex-1 rounded-xl bg-violet-600 text-white font-bold py-2 disabled:opacity-40"
              >
                Envoyer
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {!open ? (
        <button
          ref={buttonRef}
          type="button"
          onClick={() => setOpen(true)}
          className="fixed bottom-4 right-4 z-[130] flex h-14 w-14 cursor-pointer items-center justify-center rounded-full border-2 border-violet-300 bg-gradient-to-br from-violet-600 to-violet-800 text-2xl text-white shadow-lg transition-transform hover:scale-105"
          aria-label="Ouvrir le bot bien-être"
        >
          💜
        </button>
      ) : null}
    </>
  );
}
