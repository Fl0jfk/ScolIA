"use client";

import Link from "next/link";
import { Fragment, useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { usePathname } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import Image from "next/image";

type BubbleMessage = {
  role: "user" | "assistant";
  content: string;
};

type RequestDraft = {
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  subject: string;
  description: string;
};

type PendingConfirmation = {
  tool: string;
  args: Record<string, unknown>;
  summaryFr: string;
};

type BrainCta = { label: string; href: string };

function renderMessageContent(content: string) {
  const markdownLinkRegex = /\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g;
  const urlRegex = /\bhttps?:\/\/[^\s<>"')\]]+/g;
  const lines = content.split("\n");

  return lines.map((line, lineIndex) => {
    const nodes: Array<string | ReactElement> = [];
    let cursor = 0;
    let key = 0;

    const pushPlainWithUrls = (plainText: string) => {
      let plainCursor = 0;
      for (const match of plainText.matchAll(urlRegex)) {
        const rawUrl = match[0];
        const start = match.index ?? 0;
        const end = start + rawUrl.length;
        if (start > plainCursor) {
          nodes.push(plainText.slice(plainCursor, start));
        }
        nodes.push(
          <a
            key={`url_${lineIndex}_${key++}`}
            href={rawUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="text-blue-700 underline break-all"
          >
            {rawUrl}
          </a>
        );
        plainCursor = end;
      }
      if (plainCursor < plainText.length) {
        nodes.push(plainText.slice(plainCursor));
      }
    };

    for (const match of line.matchAll(markdownLinkRegex)) {
      const full = match[0];
      const label = match[1];
      const href = match[2];
      const start = match.index ?? 0;
      const end = start + full.length;

      if (start > cursor) {
        pushPlainWithUrls(line.slice(cursor, start));
      }
      nodes.push(
        <a
          key={`md_${lineIndex}_${key++}`}
          href={href}
          target="_blank"
          rel="noopener noreferrer"
          className="text-blue-700 underline break-all"
        >
          {label}
        </a>
      );
      cursor = end;
    }

    if (cursor < line.length) {
      pushPlainWithUrls(line.slice(cursor));
    }

    return (
      <Fragment key={`line_${lineIndex}`}>
        {nodes.length > 0 ? nodes : line}
        {lineIndex < lines.length - 1 && <br />}
      </Fragment>
    );
  });
}

export default function ChatbotBubble() {
  const pathname = usePathname();
  const { isSignedIn, user } = useUser();
  const demandeFormHref = isSignedIn ? "/requests" : "/faire-une-demande";
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [listening, setListening] = useState(false);
  const [input, setInput] = useState("");
  const [mounted, setMounted] = useState(false);
  const [speechSupported, setSpeechSupported] = useState(false);
  const [showRequestForm, setShowRequestForm] = useState(false);
  const [requestSending, setRequestSending] = useState(false);
  const [requestFiles, setRequestFiles] = useState<File[]>([]);
  const [requestDraft, setRequestDraft] = useState<RequestDraft>({
    firstName: "",
    lastName: "",
    email: "",
    phone: "",
    subject: "",
    description: "",
  });
  const [messages, setMessages] = useState<BubbleMessage[]>([{ role: "assistant", content: "Bonjour, je suis Nico. Posez votre question — je peux aussi réserver une salle, créer une demande, déclarer votre absence, ou consulter la feuille de semaine et les séjours." }]);
  const [conversationState, setConversationState] = useState<Record<string, unknown> | null>(null);
  const [pendingConfirmation, setPendingConfirmation] = useState<PendingConfirmation | null>(null);
  const [ctas, setCtas] = useState<BrainCta[]>([]);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const messagesRef = useRef<HTMLDivElement | null>(null);
  const scrollYRef = useRef(0);
  const hidden = useMemo(() => {
    const normalized = (pathname ?? "").toLowerCase();
    return normalized.startsWith("/sign-in") || normalized.startsWith("/sso-callback");
  }, [pathname]);
  useEffect(() => {
    setMounted(true);
    const supported = "webkitSpeechRecognition" in window || "SpeechRecognition" in window;
    setSpeechSupported(supported);
  }, []);
  useEffect(() => {
    if (!user) return;
    setRequestDraft((prev) => ({
      ...prev,
      firstName: prev.firstName || user.firstName || "",
      lastName: prev.lastName || user.lastName || "",
      email: prev.email || user.primaryEmailAddress?.emailAddress || "",
    }));
  }, [user]);
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
  }, [messages, loading, open]);
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (panelRef.current?.contains(target)) return;
      if (buttonRef.current?.contains(target)) return;
      setOpen(false);
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => { document.removeEventListener("mousedown", onPointerDown)}}, [open]);
  const canUseSpeech = speechSupported;
  const startVoiceInput = () => {
    if (!canUseSpeech || listening || loading) return;
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
  const send = async (opts?: {
    message?: string;
    confirm?: boolean;
    confirmAction?: PendingConfirmation | null;
  }) => {
    const message = (opts?.message ?? input).trim();
    const isConfirm = Boolean(opts?.confirm && opts.confirmAction?.tool);
    if ((!message && !isConfirm) || loading) return;
    if (!isConfirm) setInput("");
    if (message) {
      setMessages((prev) => [...prev, { role: "user", content: message }]);
    } else if (isConfirm) {
      setMessages((prev) => [...prev, { role: "user", content: "Confirmer" }]);
    }
    setLoading(true);
    setCtas([]);
    try {
      const res = await fetch("/api/chatbot", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          message: message || "(confirmation)",
          audience: isSignedIn ? "private" : "public",
          history: messages.slice(-10),
          conversationState: conversationState || undefined,
          confirm: isConfirm,
          confirmAction: isConfirm
            ? { tool: opts!.confirmAction!.tool, args: opts!.confirmAction!.args }
            : undefined,
        }),
      });
      const data = await res.json();
      if (data.conversationState && typeof data.conversationState === "object") {
        setConversationState(data.conversationState as Record<string, unknown>);
      }
      if (data.pendingConfirmation?.tool) {
        setPendingConfirmation(data.pendingConfirmation as PendingConfirmation);
      } else {
        setPendingConfirmation(null);
      }
      if (Array.isArray(data.ctas)) {
        setCtas(
          data.ctas.filter(
            (c: unknown): c is BrainCta =>
              Boolean(c && typeof c === "object" && typeof (c as BrainCta).href === "string"),
          ),
        );
      }
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: data.answer || data.error || "Je ne peux pas répondre pour le moment.",
        },
      ]);
    } catch {
      setMessages((prev) => [...prev, { role: "assistant", content: "Erreur réseau, merci de réessayer." }]);
    } finally {
      setLoading(false);
    }
  };

  const confirmPending = () => {
    if (!pendingConfirmation || loading) return;
    const action = pendingConfirmation;
    setPendingConfirmation(null);
    void send({ confirm: true, confirmAction: action });
  };

  const cancelPending = () => {
    setPendingConfirmation(null);
    setMessages((prev) => [
      ...prev,
      { role: "assistant", content: "Action annulée. Indiquez ce que vous souhaitez modifier." },
    ]);
  };
  const submitRequest = async () => {
    if (requestSending) return;
    setRequestSending(true);
    try {
      const fd = new FormData();
      fd.append("firstName", requestDraft.firstName);
      fd.append("lastName", requestDraft.lastName);
      fd.append("email", requestDraft.email);
      fd.append("phone", requestDraft.phone);
      fd.append("subject", requestDraft.subject);
      fd.append("description", requestDraft.description);
      requestFiles.forEach((f) => fd.append("files", f));
      const res = await fetch("/api/requests/create", {
        method: "POST",
        body: fd,
      });
      const data = await res.json();
      if (!res.ok) {
        setMessages((prev) => [...prev, { role: "assistant", content: data.error || "Impossible de créer la demande pour le moment." }]);
        return;
      }
      if (data.needsEmailVerification) {
        setMessages((prev) => [
          ...prev,
          {
            role: "assistant",
            content:
              typeof data.message === "string"
                ? data.message
                : "Un e-mail de confirmation vous a été envoyé. Ouvrez-le et cliquez sur le lien pour valider votre demande (sans ce clic, rien n’est transmis à l’équipe). Pensez à vérifier les courriers indésirables.",
          },
        ]);
        setShowRequestForm(false);
        setRequestFiles([]);
        setRequestDraft((prev) => ({ ...prev, subject: "", description: "" }));
        return;
      }
      const pj =
        typeof data.attachmentCount === "number" && data.attachmentCount > 0
          ? ` ${data.attachmentCount} pièce(s) jointe(s) incluse(s).`
          : "";
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: `Demande créée (${data.id}). Service destinataire: ${data?.assignedTo?.roleLabel || "à confirmer"}.${pj} Vous recevrez un email de suivi.`,
        },
      ]);
      setShowRequestForm(false);
      setRequestFiles([]);
      setRequestDraft((prev) => ({ ...prev, subject: "", description: "" }));
    } catch {
      setMessages((prev) => [...prev, { role: "assistant", content: "Erreur réseau lors de la création de la demande." }]);
    } finally {
      setRequestSending(false);
    }
  };
  if (hidden) return null;
  return (
    <div className="fixed inset-0 z-[120] pointer-events-none">
      {open ? (
        <div className="absolute inset-0 bg-slate-900/20 pointer-events-auto md:hidden" onClick={() => setOpen(false)} aria-hidden="true"/>
      ) : null}
      <div
        ref={panelRef}
        className={`absolute inset-0 h-[100dvh] rounded-none border-0 md:inset-auto md:right-4 md:bottom-20 md:w-[min(92vw,390px)] md:h-[570px] md:rounded-[30px] md:border md:border-white/50 bg-white/22 backdrop-blur-3xl md:shadow-[0_30px_80px_rgba(15,23,42,0.30)] overflow-hidden transition-all duration-200 pointer-events-auto ${
          open && mounted
            ? "opacity-100 scale-100 translate-y-0 pointer-events-auto"
            : "opacity-0 scale-95 translate-y-2 pointer-events-none"
        }`}
      >
          <div className="pointer-events-none absolute inset-0">
            <div className="absolute -top-16 left-1/2 -translate-x-1/2 w-[130%] h-44 bg-white/35 blur-2xl" />
            <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_0%,rgba(255,255,255,0.34),transparent_48%),radial-gradient(circle_at_100%_100%,rgba(125,211,252,0.20),transparent_35%)]" />
            <div className="absolute inset-[1px] rounded-[29px] border border-white/35" />
          </div>
          <div className="relative h-full flex flex-col">
            <div className="px-4 py-3 pt-[max(12px,env(safe-area-inset-top))] bg-gradient-to-r from-slate-950/88 via-slate-900/86 to-slate-950/88 text-white flex items-center justify-between backdrop-blur-xl border-b border-white/20">
              <p className="text-sm font-semibold tracking-wide">Nico l'assistant IA</p>
              <button type="button" onClick={() => setOpen(false)} className="text-xs opacity-80 hover:opacity-100 transition-opacity">
                Fermer
              </button>
            </div>
            <div ref={messagesRef} className="relative flex-1 min-h-0 overflow-y-auto p-3 space-y-2 bg-gradient-to-b from-white/26 via-white/14 to-slate-100/18">
              <div className="rounded-xl border border-sky-200/60 bg-white/60 p-2.5">
                <p className="text-[11px] font-semibold text-slate-800">Faire une demande</p>
                <p className="text-[10px] text-slate-600 mt-1.5 leading-relaxed">
                  Décrivez votre besoin sur une page dédiée (photos, documents). Connecté = envoi immédiat ; visiteur = confirmation par e-mail.
                </p>
                <Link
                  href={demandeFormHref}
                  className="mt-2 inline-block text-xs rounded-lg bg-sky-700 text-white px-2.5 py-1.5 font-semibold hover:bg-sky-800"
                  onClick={() => setOpen(false)}
                >
                  {isSignedIn ? "Ouvrir Demandes" : "Ouvrir le formulaire"}
                </Link>
              </div>
              {messages.map((m, i) => (
                <div
                  key={i}
                  className={`rounded-xl px-3 py-2 text-sm whitespace-pre-wrap ${
                    m.role === "user"
                      ? "bg-sky-200/55 text-sky-950 ml-8 border border-sky-200/65 backdrop-blur-md"
                      : "bg-white/55 text-slate-800 mr-8 border border-white/60 backdrop-blur-md"
                  }`}
                >
                  {renderMessageContent(m.content)}
                </div>
              ))}
              {pendingConfirmation ? (
                <div className="rounded-xl border border-amber-300/70 bg-amber-50/80 p-2.5 mr-4 space-y-2">
                  <p className="text-[11px] font-semibold text-amber-950">Confirmation requise</p>
                  <p className="text-[11px] text-amber-900 leading-relaxed whitespace-pre-wrap">
                    {pendingConfirmation.summaryFr}
                  </p>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      disabled={loading}
                      onClick={confirmPending}
                      className="text-xs rounded-lg bg-emerald-700 text-white px-2.5 py-1.5 font-semibold hover:bg-emerald-800 disabled:opacity-50"
                    >
                      Confirmer
                    </button>
                    <button
                      type="button"
                      disabled={loading}
                      onClick={cancelPending}
                      className="text-xs rounded-lg border border-slate-300 bg-white/80 px-2.5 py-1.5 font-semibold text-slate-700 disabled:opacity-50"
                    >
                      Annuler
                    </button>
                  </div>
                </div>
              ) : null}
              {ctas.length > 0 ? (
                <div className="flex flex-wrap gap-1.5 mr-4">
                  {ctas.map((c) => (
                    <Link
                      key={`${c.href}_${c.label}`}
                      href={c.href}
                      onClick={() => setOpen(false)}
                      className="text-[11px] rounded-lg bg-slate-900/90 text-white px-2.5 py-1.5 font-semibold hover:bg-black"
                    >
                      {c.label}
                    </Link>
                  ))}
                </div>
              ) : null}
              {loading ? (
                <div className="rounded-xl px-3 py-2 text-sm bg-white mr-8 border border-slate-200">
                  <div className="inline-flex items-center gap-1">
                    <span className="w-2 h-2 rounded-full bg-slate-400 animate-bounce [animation-delay:-0.2s]" />
                    <span className="w-2 h-2 rounded-full bg-slate-400 animate-bounce [animation-delay:-0.1s]" />
                    <span className="w-2 h-2 rounded-full bg-slate-400 animate-bounce" />
                  </div>
                </div>
              ) : null}
            </div>
            <div className="p-3 pb-[max(12px,env(safe-area-inset-bottom))] border-t border-white/35 bg-white/20 backdrop-blur-xl flex gap-2">
              <input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void send();
                }}
                placeholder="Écrivez votre question..."
                className="flex-1 rounded-xl border border-white/60 bg-white/60 px-3 py-2 text-base sm:text-sm text-slate-900 placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-sky-300/60"
              />
              <button
                type="button"
                onClick={startVoiceInput}
                disabled={!canUseSpeech || loading || listening}
                title={canUseSpeech ? "Dicter une question" : "Dictée vocale non supportée"}
                className="rounded-xl border border-white/60 bg-white/55 px-3 py-2 text-sm font-bold disabled:opacity-40"
              >
                {listening ? "..." : "🎤"}
              </button>
              <button
                type="button"
                onClick={() => void send()}
                disabled={loading}
                className="rounded-xl bg-slate-900/92 text-white px-3 py-2 text-sm font-semibold disabled:opacity-50 hover:bg-black transition-colors"
              >
                {loading ? "..." : "Envoyer"}
              </button>
            </div>
          </div>
      </div>
      {!open ? (
        <button
          ref={buttonRef}
          type="button"
          onClick={() => setOpen(true)}
          className="pointer-events-auto fixed bottom-4 right-4 group w-14 h-14 rounded-full border border-white/50 shadow-[0_14px_30px_rgba(15,23,42,0.38)] hover:scale-[1.05] active:scale-[0.98] transition-all overflow-hidden"
          aria-label="Ouvrir l'assistant IA"
        >
          <span className="absolute inset-[2px] rounded-full backdrop-blur-xl bg-black/10" />
          <Image src="/Nicolia.jpg" alt="Assistant IA" fill sizes="56px" className="object-cover object-top" priority/>
        </button>
      ) : null}
    </div>
  );
}
