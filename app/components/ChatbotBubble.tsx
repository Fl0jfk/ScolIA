"use client";

import Link from "next/link";
import {
  Fragment,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type DragEvent,
  type ReactElement,
} from "react";
import { usePathname, useRouter } from "next/navigation";
import { useUser } from "@clerk/nextjs";
import ScoliaAiMark from "@/app/components/ScoliaAiMark";
import {
  clearScoliaMemory,
  defaultWelcomeMessage,
  loadScoliaMemory,
  saveScoliaMemory,
  SCOLIA_AI_NAME,
  SCOLIA_AI_PAGE_PATH,
  type ScoliaMemoryMessage,
} from "@/app/lib/brain-ai/scolia-memory";

type PendingConfirmation = {
  tool: string;
  args: Record<string, unknown>;
  summaryFr: string;
};

type BrainCta = { label: string; href: string };

type PendingFile = {
  file: File;
  name: string;
};

type LayoutMode = "window" | "expanded";

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
        if (start > plainCursor) nodes.push(plainText.slice(plainCursor, start));
        nodes.push(
          <a
            key={`url_${lineIndex}_${key++}`}
            href={rawUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="text-sky-700 underline break-all"
          >
            {rawUrl}
          </a>,
        );
        plainCursor = end;
      }
      if (plainCursor < plainText.length) nodes.push(plainText.slice(plainCursor));
    };

    for (const match of line.matchAll(markdownLinkRegex)) {
      const full = match[0];
      const label = match[1];
      const href = match[2];
      const start = match.index ?? 0;
      const end = start + full.length;
      if (start > cursor) pushPlainWithUrls(line.slice(cursor, start));
      nodes.push(
        <a
          key={`md_${lineIndex}_${key++}`}
          href={href}
          target="_blank"
          rel="noopener noreferrer"
          className="text-sky-700 underline break-all"
        >
          {label}
        </a>,
      );
      cursor = end;
    }
    if (cursor < line.length) pushPlainWithUrls(line.slice(cursor));

    return (
      <Fragment key={`line_${lineIndex}`}>
        {nodes.length > 0 ? nodes : line}
        {lineIndex < lines.length - 1 && <br />}
      </Fragment>
    );
  });
}

async function uploadPdf(file: File): Promise<{ key: string; fileName: string; contentType: string }> {
  const prep = await fetch("/api/chatbot/upload", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fileName: file.name, contentType: file.type || "application/pdf" }),
  });
  const data = await prep.json();
  if (!prep.ok) throw new Error(data.error || "Upload impossible");
  const put = await fetch(data.uploadUrl, {
    method: "PUT",
    headers: { "Content-Type": file.type || "application/pdf" },
    body: file,
  });
  if (!put.ok) throw new Error("Échec envoi du PDF");
  return {
    key: String(data.key),
    fileName: String(data.fileName || file.name),
    contentType: String(data.contentType || "application/pdf"),
  };
}

type Props = {
  /** Mode page dédiée (/scolia-ai) — pas de bulle flottante. */
  pageMode?: boolean;
};

export default function ChatbotBubble({ pageMode = false }: Props) {
  const pathname = usePathname();
  const router = useRouter();
  const { isSignedIn } = useUser();
  const [open, setOpen] = useState(pageMode);
  const [layout, setLayout] = useState<LayoutMode>(pageMode ? "expanded" : "window");
  const [loading, setLoading] = useState(false);
  const [listening, setListening] = useState(false);
  const [interimSpeech, setInterimSpeech] = useState("");
  const [input, setInput] = useState("");
  const [mounted, setMounted] = useState(false);
  const [speechSupported, setSpeechSupported] = useState(false);
  const [memoryReady, setMemoryReady] = useState(false);
  const [messages, setMessages] = useState<ScoliaMemoryMessage[]>([defaultWelcomeMessage()]);
  const [conversationState, setConversationState] = useState<Record<string, unknown> | null>(null);
  const [pendingConfirmation, setPendingConfirmation] = useState<PendingConfirmation | null>(null);
  const [ctas, setCtas] = useState<BrainCta[]>([]);
  const [pendingFiles, setPendingFiles] = useState<PendingFile[]>([]);
  const [dragOver, setDragOver] = useState(false);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const messagesRef = useRef<HTMLDivElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const recognitionRef = useRef<{ stop: () => void } | null>(null);
  const scrollYRef = useRef(0);

  const hidden = useMemo(() => {
    if (pageMode) return false;
    const normalized = (pathname ?? "").toLowerCase();
    if (normalized === SCOLIA_AI_PAGE_PATH || normalized.startsWith(`${SCOLIA_AI_PAGE_PATH}/`)) {
      return true;
    }
    return normalized.startsWith("/sign-in") || normalized.startsWith("/sso-callback");
  }, [pathname, pageMode]);

  useEffect(() => {
    setMounted(true);
    const supported = "webkitSpeechRecognition" in window || "SpeechRecognition" in window;
    setSpeechSupported(supported);
    const mem = loadScoliaMemory();
    if (mem?.messages?.length) {
      setMessages(mem.messages);
      setConversationState(mem.conversationState);
      setPendingConfirmation(mem.pendingConfirmation);
    }
    setMemoryReady(true);
  }, []);

  useEffect(() => {
    if (!memoryReady) return;
    saveScoliaMemory({
      messages,
      conversationState,
      pendingConfirmation,
    });
  }, [messages, conversationState, pendingConfirmation, memoryReady]);

  useEffect(() => {
    if (!open || pageMode || layout !== "expanded") return;
    const body = document.body;
    const html = document.documentElement;
    const previousBodyOverflow = body.style.overflow;
    scrollYRef.current = window.scrollY;
    body.style.overflow = "hidden";
    html.style.overscrollBehavior = "none";
    return () => {
      body.style.overflow = previousBodyOverflow;
      html.style.overscrollBehavior = "";
      window.scrollTo(0, scrollYRef.current);
    };
  }, [open, layout, pageMode]);

  useEffect(() => {
    if (!open) return;
    const el = messagesRef.current;
    if (!el) return;
    el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }, [messages, loading, open, listening, pendingConfirmation]);

  useEffect(() => {
    if (!open || pageMode || layout === "expanded") return;
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (panelRef.current?.contains(target)) return;
      if (buttonRef.current?.contains(target)) return;
      setOpen(false);
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [open, pageMode, layout]);

  const stopVoice = useCallback(() => {
    try {
      recognitionRef.current?.stop();
    } catch {
      /* ignore */
    }
    recognitionRef.current = null;
    setListening(false);
    setInterimSpeech("");
  }, []);

  const startVoiceInput = () => {
    if (!speechSupported || loading) return;
    if (listening) {
      stopVoice();
      return;
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const SpeechRecognitionCtor = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (!SpeechRecognitionCtor) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const recognition = new (SpeechRecognitionCtor as any)();
    recognition.lang = "fr-FR";
    recognition.interimResults = true;
    recognition.continuous = false;
    recognition.maxAlternatives = 1;
    recognitionRef.current = recognition;
    setListening(true);
    setInterimSpeech("");
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    recognition.onresult = (event: any) => {
      let interim = "";
      let finalText = "";
      for (let i = event.resultIndex; i < event.results.length; i += 1) {
        const t = event.results[i][0]?.transcript || "";
        if (event.results[i].isFinal) finalText += t;
        else interim += t;
      }
      if (interim) setInterimSpeech(interim.trim());
      if (finalText.trim()) {
        setInput((prev) => (prev ? `${prev} ${finalText.trim()}` : finalText.trim()));
        setInterimSpeech("");
      }
    };
    recognition.onerror = () => stopVoice();
    recognition.onend = () => {
      recognitionRef.current = null;
      setListening(false);
      setInterimSpeech("");
    };
    recognition.start();
  };

  const send = async (opts?: {
    message?: string;
    confirm?: boolean;
    confirmAction?: PendingConfirmation | null;
    files?: PendingFile[];
  }) => {
    const message = (opts?.message ?? input).trim();
    const isConfirm = Boolean(opts?.confirm && opts.confirmAction?.tool);
    const filesToSend = opts?.files ?? pendingFiles;
    if ((!message && !isConfirm && filesToSend.length === 0) || loading) return;
    if (!isConfirm) {
      setInput("");
      setPendingFiles([]);
    }
    const userLabel = message
      || (isConfirm ? "Confirmer" : "")
      || (filesToSend.length ? `(fichier : ${filesToSend.map((f) => f.name).join(", ")})` : "");
    if (userLabel) {
      setMessages((prev) => [...prev, { role: "user", content: userLabel }]);
    }
    setLoading(true);
    setCtas([]);
    stopVoice();
    try {
      let attachments: Array<{ key: string; fileName: string; contentType: string }> | undefined;
      if (filesToSend.length > 0) {
        attachments = [];
        for (const pf of filesToSend) {
          attachments.push(await uploadPdf(pf.file));
        }
      }

      const res = await fetch("/api/chatbot", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          message: message || (isConfirm ? "(confirmation)" : "Voici un document joint."),
          audience: isSignedIn ? "private" : "public",
          history: messages.slice(-10),
          conversationState: conversationState || undefined,
          confirm: isConfirm,
          confirmAction: isConfirm
            ? { tool: opts!.confirmAction!.tool, args: opts!.confirmAction!.args }
            : undefined,
          attachments,
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
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: "Erreur réseau, merci de réessayer." },
      ]);
    } finally {
      setLoading(false);
    }
  };

  const confirmPending = () => {
    if (!pendingConfirmation || loading) return;
    const action = pendingConfirmation;
    const files = pendingFiles;
    setPendingConfirmation(null);
    void send({ confirm: true, confirmAction: action, files });
  };

  const cancelPending = () => {
    setPendingConfirmation(null);
    setMessages((prev) => [
      ...prev,
      { role: "assistant", content: "Action annulée. Dites-moi ce que vous voulez faire ensuite." },
    ]);
  };

  const modifyPending = () => {
    if (!pendingConfirmation) return;
    const draft = pendingConfirmation.args;
    setConversationState((prev) => ({
      ...(prev || {}),
      slots: {
        ...((prev?.slots as Record<string, unknown>) || {}),
        pendingEdit: draft,
        pendingEditTool: pendingConfirmation.tool,
      },
      pendingConfirmation: null,
    }));
    setPendingConfirmation(null);
    setMessages((prev) => [
      ...prev,
      {
        role: "assistant",
        content:
          "Que souhaitez-vous modifier dans votre demande ? (ex. le nombre, la classe, l’établissement, le PDF…)",
      },
    ]);
  };

  const onPickFiles = (list: FileList | null) => {
    if (!list?.length) return;
    const next: PendingFile[] = [];
    for (const file of Array.from(list)) {
      if (file.type !== "application/pdf" && !file.name.toLowerCase().endsWith(".pdf")) {
        setMessages((prev) => [
          ...prev,
          { role: "assistant", content: `« ${file.name} » n’est pas un PDF. Seuls les PDF sont acceptés pour le moment.` },
        ]);
        continue;
      }
      next.push({ file, name: file.name });
    }
    if (next.length) setPendingFiles((prev) => [...prev, ...next].slice(0, 3));
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  const onDragEnter = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!isSignedIn) return;
    if (Array.from(e.dataTransfer.types || []).includes("Files")) setDragOver(true);
  };

  const onDragOver = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!isSignedIn) return;
    e.dataTransfer.dropEffect = "copy";
    setDragOver(true);
  };

  const onDragLeave = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const related = e.relatedTarget as Node | null;
    if (related && e.currentTarget.contains(related)) return;
    setDragOver(false);
  };

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);
    if (!isSignedIn) {
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: "Connectez-vous pour déposer un PDF dans ScolIA." },
      ]);
      return;
    }
    onPickFiles(e.dataTransfer.files);
  };

  const resetConversation = () => {
    clearScoliaMemory();
    setMessages([defaultWelcomeMessage()]);
    setConversationState(null);
    setPendingConfirmation(null);
    setCtas([]);
    setPendingFiles([]);
    setInput("");
  };

  const openExpanded = () => {
    setLayout("expanded");
    setOpen(true);
  };

  const collapseToWindow = () => {
    if (pageMode) {
      router.push("/dashboard");
      return;
    }
    setLayout("window");
  };

  const openDedicatedPage = () => {
    router.push(SCOLIA_AI_PAGE_PATH);
  };

  if (hidden) return null;

  const isExpanded = pageMode || layout === "expanded";

  const renderChatBody = () => (
    <div
      className={`relative h-full flex flex-col ${dragOver ? "ring-2 ring-emerald-400/60 ring-inset" : ""}`}
      onDragEnter={onDragEnter}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
    >
      {dragOver ? (
        <div className="pointer-events-none absolute inset-0 z-20 flex items-center justify-center bg-emerald-50/80 backdrop-blur-[2px]">
          <div className="rounded-2xl border-2 border-dashed border-emerald-500 bg-white/90 px-6 py-5 text-center shadow-lg">
            <p className="text-sm font-semibold text-emerald-900">Déposez votre PDF ici</p>
            <p className="mt-1 text-[11px] text-emerald-800/80">ScolIA l’ajoutera à la conversation</p>
          </div>
        </div>
      ) : null}
      {/* Header */}
      <div
        className={`flex items-center justify-between gap-3 border-b ${
          isExpanded
            ? "px-5 py-4 pt-[max(16px,env(safe-area-inset-top))] border-white/10 bg-transparent"
            : "px-3 py-2.5 pt-[max(10px,env(safe-area-inset-top))] border-white/15 bg-slate-950/80 text-white backdrop-blur-xl"
        }`}
      >
        <div className="flex items-center gap-2 min-w-0">
          {!pageMode && !isExpanded ? (
            <div className="flex items-center gap-1.5 pr-1" aria-label="Contrôles fenêtre">
              <button
                type="button"
                title="Fermer"
                onClick={() => setOpen(false)}
                className="h-3 w-3 rounded-full bg-[#ff5f57] hover:brightness-110 shadow-sm"
              />
              <button
                type="button"
                title="Réduire"
                onClick={() => setOpen(false)}
                className="h-3 w-3 rounded-full bg-[#febc2e] hover:brightness-110 shadow-sm"
              />
              <button
                type="button"
                title="Agrandir"
                onClick={openExpanded}
                className="h-3 w-3 rounded-full bg-[#28c840] hover:brightness-110 shadow-sm"
              />
            </div>
          ) : null}
          <div className="min-w-0">
            <ScoliaAiMark
              size={isExpanded ? "lg" : "sm"}
              inverted={!isExpanded && !pageMode}
            />
            {isExpanded ? (
              <p className="text-[11px] text-slate-500 truncate mt-1">Assistant de votre établissement</p>
            ) : null}
          </div>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {isExpanded && !pageMode ? (
            <button
              type="button"
              onClick={collapseToWindow}
              className="text-[11px] rounded-full px-2.5 py-1 border border-slate-200 bg-white/70 text-slate-700 hover:bg-white"
            >
              Réduire
            </button>
          ) : null}
          {isExpanded ? (
            <button
              type="button"
              onClick={resetConversation}
              className="text-[11px] rounded-full px-2.5 py-1 border border-slate-200 bg-white/70 text-slate-700 hover:bg-white"
            >
              Nouvelle conversation
            </button>
          ) : null}
          {!isExpanded ? (
            <button
              type="button"
              onClick={openDedicatedPage}
              title="Ouvrir en page complète"
              className="text-[10px] opacity-80 hover:opacity-100"
            >
              Page
            </button>
          ) : pageMode ? (
            <Link href="/dashboard" className="text-[11px] text-slate-600 hover:text-slate-900">
              Fermer
            </Link>
          ) : (
            <button
              type="button"
              onClick={() => {
                setOpen(false);
                setLayout("window");
              }}
              className="text-[11px] text-slate-600 hover:text-slate-900"
            >
              Fermer
            </button>
          )}
        </div>
      </div>

      {/* Messages */}
      <div
        ref={messagesRef}
        className={`relative flex-1 min-h-0 overflow-y-auto ${
          isExpanded
            ? "px-4 sm:px-8 py-6 space-y-4"
            : "p-3 space-y-2 bg-gradient-to-b from-white/30 via-white/12 to-slate-100/20"
        }`}
      >
        {isExpanded ? (
          <div className="mx-auto w-full max-w-2xl space-y-4">
            {messages.length <= 1 ? (
              <div className="flex flex-col items-center justify-center py-10 sm:py-16 text-center">
                <ScoliaAiMark size="lg" />
                <p className="mt-4 max-w-md text-sm text-slate-600 leading-relaxed">
                  Posez une question, dictez au micro, ou glissez un PDF ici quand j’en ai besoin.
                </p>
              </div>
            ) : null}
            {(messages.length <= 1 ? [] : messages).map((m, i) => (
              <div
                key={i}
                className={`flex ${m.role === "user" ? "justify-end" : "justify-start"}`}
              >
                <div
                  className={`max-w-[92%] rounded-3xl px-4 py-3 text-[15px] leading-relaxed whitespace-pre-wrap ${
                    m.role === "user"
                      ? "bg-slate-900 text-white rounded-br-lg"
                      : "bg-white/80 text-slate-800 border border-slate-200/70 shadow-sm rounded-bl-lg backdrop-blur-md"
                  }`}
                >
                  {renderMessageContent(m.content)}
                </div>
              </div>
            ))}
            {renderExtras()}
          </div>
        ) : (
          <>
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
            {renderExtras()}
          </>
        )}
      </div>

      {/* Composer */}
      <div
        className={`border-t backdrop-blur-xl ${
          isExpanded
            ? "border-slate-200/60 bg-white/55 px-4 sm:px-8 py-4 pb-[max(16px,env(safe-area-inset-bottom))]"
            : "border-white/35 bg-white/25 p-3 pb-[max(12px,env(safe-area-inset-bottom))]"
        }`}
      >
        <div className={isExpanded ? "mx-auto w-full max-w-2xl" : ""}>
          {listening ? (
            <div className="mb-2 flex items-center gap-3 rounded-2xl border border-rose-200/70 bg-rose-50/90 px-3 py-2">
              <span className="relative flex h-8 w-8 items-center justify-center">
                <span className="absolute inset-0 rounded-full bg-rose-400/30 animate-ping" />
                <span className="absolute inset-1 rounded-full bg-rose-400/40 animate-pulse" />
                <span className="relative h-3 w-3 rounded-full bg-rose-500" />
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-[11px] font-semibold text-rose-900">Écoute en cours…</p>
                <p className="text-[11px] text-rose-800/80 truncate">
                  {interimSpeech || "Parlez, je vous écoute"}
                </p>
              </div>
              <div className="flex items-end gap-0.5 h-6" aria-hidden>
                {[0, 1, 2, 3, 4].map((i) => (
                  <span
                    key={i}
                    className="w-1 rounded-full bg-rose-500 animate-pulse"
                    style={{
                      height: `${8 + ((i * 5) % 14)}px`,
                      animationDelay: `${i * 0.12}s`,
                    }}
                  />
                ))}
              </div>
              <button
                type="button"
                onClick={stopVoice}
                className="text-[11px] font-semibold text-rose-900 underline"
              >
                Stop
              </button>
            </div>
          ) : null}

          {pendingFiles.length > 0 ? (
            <div className="mb-2 flex flex-wrap gap-1.5">
              {pendingFiles.map((f, idx) => (
                <span
                  key={`${f.name}_${idx}`}
                  className="inline-flex items-center gap-1.5 rounded-full border border-slate-200 bg-white/90 px-2.5 py-1 text-[11px] text-slate-700"
                >
                  PDF · {f.name}
                  <button
                    type="button"
                    className="text-slate-400 hover:text-slate-700"
                    onClick={() => setPendingFiles((prev) => prev.filter((_, i) => i !== idx))}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
          ) : null}

          <div
            className={`flex items-end gap-2 rounded-2xl border ${
              isExpanded
                ? "border-slate-200 bg-white shadow-[0_10px_40px_rgba(15,23,42,0.08)] px-3 py-2"
                : "border-white/60 bg-white/70 px-2 py-1.5"
            }`}
          >
            <input
              ref={fileInputRef}
              type="file"
              accept="application/pdf,.pdf"
              className="hidden"
              onChange={(e) => onPickFiles(e.target.files)}
            />
            <button
              type="button"
              disabled={loading || !isSignedIn}
              title={isSignedIn ? "Joindre un PDF" : "Connectez-vous pour joindre un PDF"}
              onClick={() => fileInputRef.current?.click()}
              className="shrink-0 rounded-xl px-2 py-2 text-sm text-slate-600 hover:bg-slate-100 disabled:opacity-40"
            >
              📎
            </button>
            <textarea
              value={input}
              rows={isExpanded ? 2 : 1}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  void send();
                }
              }}
              placeholder={
                listening
                  ? "Écoute…"
                  : isSignedIn
                    ? "Écrivez, dictez, joignez ou glissez un PDF…"
                    : "Écrivez ou dictez…"
              }
              className="flex-1 resize-none bg-transparent px-1 py-2 text-base sm:text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none max-h-32"
            />
            <button
              type="button"
              onClick={startVoiceInput}
              disabled={!speechSupported || loading}
              title={speechSupported ? (listening ? "Arrêter l’écoute" : "Dicter") : "Dictée non supportée"}
              className={`shrink-0 rounded-xl px-2.5 py-2 text-sm font-bold disabled:opacity-40 ${
                listening ? "bg-rose-500 text-white" : "hover:bg-slate-100 text-slate-700"
              }`}
            >
              🎤
            </button>
            <button
              type="button"
              onClick={() => void send()}
              disabled={loading || (!input.trim() && pendingFiles.length === 0)}
              className="shrink-0 rounded-xl bg-slate-900 text-white px-3 py-2 text-sm font-semibold disabled:opacity-50 hover:bg-black"
            >
              {loading ? "…" : "Envoyer"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );

  function renderExtras() {
    return (
      <>
        {pendingConfirmation ? (
          <div
            className={`rounded-2xl border border-amber-300/70 bg-amber-50/90 p-3 space-y-2 ${
              isExpanded ? "" : "mr-4"
            }`}
          >
            <p className="text-[11px] font-semibold text-amber-950">Confirmation requise</p>
            <p className="text-[12px] text-amber-900 leading-relaxed whitespace-pre-wrap">
              {pendingConfirmation.summaryFr}
            </p>
            {pendingFiles.length > 0 ? (
              <p className="text-[11px] text-amber-800">
                PDF prêt à joindre : {pendingFiles.map((f) => f.name).join(", ")}
              </p>
            ) : null}
            <div className="flex flex-wrap gap-2">
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
                onClick={modifyPending}
                className="text-xs rounded-lg bg-sky-700 text-white px-2.5 py-1.5 font-semibold hover:bg-sky-800 disabled:opacity-50"
              >
                Modifier
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
          <div className={`flex flex-wrap gap-1.5 ${isExpanded ? "" : "mr-4"}`}>
            {ctas.map((c) => (
              <Link
                key={`${c.href}_${c.label}`}
                href={c.href}
                onClick={() => {
                  if (!pageMode) setOpen(false);
                }}
                className="text-[11px] rounded-lg bg-slate-900/90 text-white px-2.5 py-1.5 font-semibold hover:bg-black"
              >
                {c.label}
              </Link>
            ))}
          </div>
        ) : null}
        {loading ? (
          <div
            className={`rounded-2xl px-3 py-2 text-sm bg-white/80 border border-slate-200 ${
              isExpanded ? "w-fit" : "mr-8"
            }`}
          >
            <div className="inline-flex items-center gap-1">
              <span className="w-2 h-2 rounded-full bg-slate-400 animate-bounce [animation-delay:-0.2s]" />
              <span className="w-2 h-2 rounded-full bg-slate-400 animate-bounce [animation-delay:-0.1s]" />
              <span className="w-2 h-2 rounded-full bg-slate-400 animate-bounce" />
            </div>
          </div>
        ) : null}
      </>
    );
  }

  if (pageMode) {
    return (
      <div className="min-h-[100dvh] w-full bg-[radial-gradient(ellipse_at_top,_#e8f0ff_0%,_#f8fafc_45%,_#eef2ff_100%)]">
        <div className="mx-auto flex h-[100dvh] w-full max-w-5xl flex-col">{renderChatBody()}</div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 z-[120] pointer-events-none">
      {open && layout === "expanded" ? (
        <div className="absolute inset-0 pointer-events-auto bg-[radial-gradient(ellipse_at_top,_rgba(232,240,255,0.97)_0%,_rgba(248,250,252,0.98)_50%,_rgba(238,242,255,0.97)_100%)] backdrop-blur-xl">
          <div className="mx-auto flex h-[100dvh] w-full max-w-5xl flex-col">{renderChatBody()}</div>
        </div>
      ) : null}

      {open && layout === "window" ? (
        <div className="absolute inset-0 bg-slate-900/20 pointer-events-auto md:hidden" onClick={() => setOpen(false)} aria-hidden />
      ) : null}

      <div
        ref={panelRef}
        className={`absolute inset-0 h-[100dvh] rounded-none border-0 md:inset-auto md:right-4 md:bottom-20 md:w-[min(92vw,400px)] md:h-[580px] md:rounded-[22px] md:border md:border-white/50 bg-white/25 backdrop-blur-3xl md:shadow-[0_30px_80px_rgba(15,23,42,0.30)] overflow-hidden transition-all duration-200 pointer-events-auto ${
          open && mounted && layout === "window"
            ? "opacity-100 scale-100 translate-y-0"
            : "opacity-0 scale-95 translate-y-2 pointer-events-none"
        }`}
      >
        <div className="pointer-events-none absolute inset-0">
          <div className="absolute -top-16 left-1/2 -translate-x-1/2 w-[130%] h-44 bg-white/35 blur-2xl" />
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_0%,rgba(255,255,255,0.34),transparent_48%),radial-gradient(circle_at_100%_100%,rgba(125,211,252,0.20),transparent_35%)]" />
          <div className="absolute inset-[1px] rounded-[21px] border border-white/35" />
        </div>
        <div className="relative h-full">{renderChatBody()}</div>
      </div>

      {!open ? (
        <button
          ref={buttonRef}
          type="button"
          onClick={() => {
            setLayout("window");
            setOpen(true);
          }}
          className="pointer-events-auto fixed bottom-4 right-4 group h-14 min-w-14 px-4 rounded-full border border-white/50 shadow-[0_14px_30px_rgba(15,23,42,0.38)] hover:scale-[1.03] active:scale-[0.98] transition-all overflow-hidden bg-slate-950 text-white flex items-center justify-center gap-2"
          aria-label={`Ouvrir ${SCOLIA_AI_NAME}`}
        >
          <span className="absolute inset-0 bg-[radial-gradient(circle_at_30%_20%,rgba(125,211,252,0.35),transparent_55%)]" />
          <span className="relative scale-90">
            <ScoliaAiMark size="sm" inverted />
          </span>
        </button>
      ) : null}
    </div>
  );
}
