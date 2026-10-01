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
import { useSessionUser } from "@/app/hooks/useAppUser";
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
import { SCOLIA_ASK_EVENT, SCOLIA_OPEN_EVENT, type ScoliaAskDetail, type ScoliaOpenDetail } from "@/app/lib/brain-ai/scolia-ask";
import {
  ScoliaUrlModal,
  useScoliaClientActions,
} from "@/app/components/scolia/ScoliaClientActions";
import { useEleveDossierModalOptional } from "@/app/components/shell/EleveDossierModalProvider";
import type { BrainClientAction } from "@/app/lib/brain-ai/types";

type PendingConfirmation = {
  tool: string;
  args: Record<string, unknown>;
  summaryFr: string;
};

type PendingChoices = {
  tool: string;
  field: string;
  promptFr: string;
  options: Array<{ value: string; label: string }>;
  draftArgs: Record<string, unknown>;
  selectionType?: "single" | "multi" | "date" | "text";
};

type PendingFileUpload = {
  tool: string;
  promptFr: string;
  draftArgs: Record<string, unknown>;
  optional?: boolean;
  accept?: string;
};

type BrainCta = { label: string; href: string; preview?: boolean };

type PendingFile = {
  file: File;
  name: string;
};

function linkPillClass(onDark: boolean): string {
  return onDark
    ? "inline-flex max-w-full items-center gap-1 truncate rounded-full bg-white/15 px-2.5 py-0.5 text-[13px] font-semibold text-[var(--dash-lime)] no-underline ring-1 ring-white/20 transition hover:bg-white/25"
    : "inline-flex max-w-full items-center gap-1 truncate rounded-full bg-[color:var(--dash-lime)]/55 px-2.5 py-0.5 text-[13px] font-semibold text-[var(--dash-ink)] no-underline ring-1 ring-black/5 transition hover:bg-[color:var(--dash-lime)]";
}

function shortUrlLabel(rawUrl: string): string {
  try {
    const u = new URL(rawUrl);
    const path = u.pathname === "/" ? "" : u.pathname;
    const label = `${u.hostname}${path}`;
    return label.length > 42 ? `${label.slice(0, 40)}…` : label;
  } catch {
    return rawUrl.length > 42 ? `${rawUrl.slice(0, 40)}…` : rawUrl;
  }
}

function renderMessageContent(content: string, onDark = false) {
  const markdownLinkRegex = /\[([^\]]+)\]\((https?:\/\/[^\s)]+|\/[^\s)]+)\)/g;
  const urlRegex = /\bhttps?:\/\/[^\s<>"')\]]+/g;
  const lines = content.split("\n");
  const pill = linkPillClass(onDark);

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
            className={pill}
            title={rawUrl}
          >
            {shortUrlLabel(rawUrl)}
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
      const external = href.startsWith("http");
      nodes.push(
        <a
          key={`md_${lineIndex}_${key++}`}
          href={href}
          {...(external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
          className={pill}
          title={href}
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
  /** Intégré dans un layout parent (hub dashboard) — hauteur 100% au lieu de 100dvh. */
  embedded?: boolean;
};

export default function ChatbotBubble({ pageMode = false, embedded = false }: Props) {
  const pathname = usePathname();
  const router = useRouter();
  const { isSignedIn } = useSessionUser();
  const { runActions, urlModal, closeUrlModal, openUrlPreview } = useScoliaClientActions();
  const dossierModal = useEleveDossierModalOptional();
  const [open, setOpen] = useState(pageMode);
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
  const [pendingChoices, setPendingChoices] = useState<PendingChoices | null>(null);
  const [pendingFileUpload, setPendingFileUpload] = useState<PendingFileUpload | null>(null);
  const [choiceDraft, setChoiceDraft] = useState("");
  const [choiceMulti, setChoiceMulti] = useState<string[]>([]);
  const [ctas, setCtas] = useState<BrainCta[]>([]);
  const [pendingFiles, setPendingFiles] = useState<PendingFile[]>([]);
  const [dragOver, setDragOver] = useState(false);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const messagesRef = useRef<HTMLDivElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const recognitionRef = useRef<{ stop: () => void } | null>(null);
  const scrollYRef = useRef(0);
  /** Texte final dicté — envoyé auto quand le micro s’arrête. */
  const voiceFinalRef = useRef("");
  const sendRef = useRef<(opts?: { message?: string }) => void>(() => undefined);

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
      setPendingChoices(mem.pendingChoices ?? null);
    }
    setMemoryReady(true);
  }, []);

  useEffect(() => {
    if (!memoryReady) return;
    saveScoliaMemory({
      messages,
      conversationState,
      pendingConfirmation,
      pendingChoices,
    });
  }, [messages, conversationState, pendingConfirmation, pendingChoices, memoryReady]);

  useEffect(() => {
    if (!open || pageMode) return;
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
  }, [open, pageMode]);

  useEffect(() => {
    if (!open) return;
    const el = messagesRef.current;
    if (!el) return;
    el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }, [messages, loading, open, listening, pendingConfirmation, pendingChoices]);

  useEffect(() => {
    if (!open || pageMode) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, pageMode]);

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
      // Arrêt manuel : onend enverra le texte dicté.
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
    voiceFinalRef.current = "";
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
        const chunk = finalText.trim();
        voiceFinalRef.current = voiceFinalRef.current
          ? `${voiceFinalRef.current} ${chunk}`
          : chunk;
        setInput(voiceFinalRef.current);
        setInterimSpeech("");
      }
    };
    recognition.onerror = () => {
      recognitionRef.current = null;
      setListening(false);
      setInterimSpeech("");
      // En cas d’erreur après du texte final, on envoie quand même.
      const text = voiceFinalRef.current.trim();
      voiceFinalRef.current = "";
      if (text) sendRef.current({ message: text });
    };
    recognition.onend = () => {
      recognitionRef.current = null;
      setListening(false);
      setInterimSpeech("");
      const text = voiceFinalRef.current.trim();
      voiceFinalRef.current = "";
      if (text) {
        setInput("");
        sendRef.current({ message: text });
      }
    };
    recognition.start();
  };

  const send = async (opts?: {
    message?: string;
    confirm?: boolean;
    confirmAction?: PendingConfirmation | null;
    choiceApply?: {
      tool: string;
      field: string;
      value?: string;
      values?: string[];
      draftArgs: Record<string, unknown>;
    } | null;
    fileApply?: {
      tool: string;
      draftArgs: Record<string, unknown>;
      skipPdf?: boolean;
    } | null;
    files?: PendingFile[];
  }) => {
    const message = (opts?.message ?? input).trim();
    const isConfirm = Boolean(opts?.confirm && opts.confirmAction?.tool);
    const isChoice = Boolean(opts?.choiceApply?.tool);
    const isFileApply = Boolean(opts?.fileApply?.tool);
    const filesToSend = opts?.files ?? pendingFiles;
    if ((!message && !isConfirm && !isChoice && !isFileApply && filesToSend.length === 0) || loading) return;
    // Évite un double envoi si stopVoice déclenche onend pendant un send manuel.
    voiceFinalRef.current = "";
    if (!isConfirm && !isChoice && !isFileApply) {
      setInput("");
      setPendingFiles([]);
    }
    const userLabel = message
      || (isConfirm ? "Confirmer" : "")
      || (isChoice
        ? (opts!.choiceApply!.values?.length
            ? opts!.choiceApply!.values.join(", ")
            : opts!.choiceApply!.value || "Choix")
        : "")
      || (isFileApply
        ? (opts!.fileApply!.skipPdf ? "Continuer sans PDF" : "PDF joint")
        : "")
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
        setPendingFiles([]);
      }

      const res = await fetch("/api/chatbot", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          message:
            message ||
            (isConfirm
              ? "(confirmation)"
              : isChoice
                ? "(choix)"
                : isFileApply
                  ? "(fichier)"
                  : "Voici un document joint."),
          audience: isSignedIn ? "private" : "public",
          history: messages.slice(-10),
          conversationState: conversationState || undefined,
          confirm: isConfirm,
          confirmAction: isConfirm
            ? { tool: opts!.confirmAction!.tool, args: opts!.confirmAction!.args }
            : undefined,
          choiceApply: isChoice ? opts!.choiceApply : undefined,
          fileApply: isFileApply ? opts!.fileApply : undefined,
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
      if (data.pendingChoices?.tool && data.pendingChoices?.field) {
        setPendingChoices(data.pendingChoices as PendingChoices);
        setChoiceDraft("");
        setChoiceMulti([]);
      } else {
        setPendingChoices(null);
        setChoiceDraft("");
        setChoiceMulti([]);
      }
      if (data.pendingFileUpload?.tool) {
        setPendingFileUpload(data.pendingFileUpload as PendingFileUpload);
      } else {
        setPendingFileUpload(null);
      }
      if (Array.isArray(data.ctas)) {
        setCtas(
          data.ctas
            .filter(
              (c: unknown): c is BrainCta =>
                Boolean(c && typeof c === "object" && typeof (c as BrainCta).href === "string"),
            )
            .map((c: BrainCta) => ({
              label: String(c.label || "Ouvrir"),
              href: c.href,
              ...(c.preview ? { preview: true as const } : {}),
            })),
        );
      } else {
        setCtas([]);
      }
      const answerText = data.answer || data.error || "Je ne peux pas répondre pour le moment.";
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: answerText,
        },
      ]);
      if (Array.isArray(data.clientActions)) {
        runActions(data.clientActions as BrainClientAction[]);
      }
      // Persistance serveur (best-effort) — historique multi-appareils
      if (isSignedIn && data.conversationState?.conversationId) {
        const convId = String(data.conversationState.conversationId);
        const toPersist: Array<{ role: "user" | "assistant"; content: string }> = [];
        if (userLabel) toPersist.push({ role: "user", content: userLabel });
        toPersist.push({ role: "assistant", content: answerText });
        void fetch(`/api/chatbot/conversations/${encodeURIComponent(convId)}/messages`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            messages: toPersist,
            state: data.conversationState,
            titleHint: userLabel || undefined,
          }),
        }).catch(() => {
          /* ignore */
        });
      }
    } catch {
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: "Erreur réseau, merci de réessayer." },
      ]);
    } finally {
      setLoading(false);
    }
  };

  sendRef.current = send;

  useEffect(() => {
    const onAsk = (ev: Event) => {
      const detail = (ev as CustomEvent<ScoliaAskDetail>).detail;
      const prompt = detail?.prompt?.trim();
      if (!prompt) return;
      setOpen(true);
      if (detail.autoSend === false) {
        setInput(prompt);
        return;
      }
      window.setTimeout(() => {
        void sendRef.current({ message: prompt });
      }, 50);
    };
    const onOpen = (ev: Event) => {
      const detail = (ev as CustomEvent<ScoliaOpenDetail>).detail;
      setOpen(true);
      if (detail?.draft?.trim()) setInput(detail.draft.trim());
    };
    window.addEventListener(SCOLIA_ASK_EVENT, onAsk as EventListener);
    window.addEventListener(SCOLIA_OPEN_EVENT, onOpen as EventListener);
    return () => {
      window.removeEventListener(SCOLIA_ASK_EVENT, onAsk as EventListener);
      window.removeEventListener(SCOLIA_OPEN_EVENT, onOpen as EventListener);
    };
  }, []);

  const confirmPending = () => {
    if (!pendingConfirmation || loading) return;
    const action = pendingConfirmation;
    const files = pendingFiles;
    setPendingConfirmation(null);
    void send({ confirm: true, confirmAction: action, files });
  };

  const cancelPending = () => {
    setPendingConfirmation(null);
    setPendingChoices(null);
    setChoiceDraft("");
    setChoiceMulti([]);
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
      pendingChoices: null,
    }));
    setPendingConfirmation(null);
    setPendingChoices(null);
    setMessages((prev) => [
      ...prev,
      {
        role: "assistant",
        content:
          "Que souhaitez-vous modifier dans votre demande ? (ex. le nombre, la classe, l’établissement, le PDF…)",
      },
    ]);
  };

  const submitChoice = (value?: string, values?: string[]) => {
    if (!pendingChoices || loading) return;
    const label =
      values?.length
        ? values
            .map((v) => pendingChoices.options.find((o) => o.value === v)?.label || v)
            .join(", ")
        : pendingChoices.options.find((o) => o.value === value)?.label || value || "";
    const payload = {
      tool: pendingChoices.tool,
      field: pendingChoices.field,
      value,
      values,
      draftArgs: pendingChoices.draftArgs,
    };
    setPendingChoices(null);
    setChoiceDraft("");
    setChoiceMulti([]);
    void send({ message: label, choiceApply: payload });
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
    setPendingChoices(null);
    setChoiceDraft("");
    setChoiceMulti([]);
    setCtas([]);
    setPendingFiles([]);
    setInput("");
  };

  const closeChat = () => {
    if (pageMode) {
      router.push("/dashboard");
      return;
    }
    setOpen(false);
  };

  if (hidden) return null;

  /** Toujours le mode conversation large (page dédiée ou modale centrale). */
  const isExpanded = true;

  const renderChatBody = () => (
    <div
      className={`relative flex h-full flex-col ${dragOver ? "ring-2 ring-[color:var(--dash-lime)]/70 ring-inset" : ""}`}
      onDragEnter={onDragEnter}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
    >
      {dragOver ? (
        <div className="pointer-events-none absolute inset-0 z-20 flex items-center justify-center bg-[color:var(--dash-soft-muted)]/90 backdrop-blur-[2px]">
          <div className="rounded-[1.5rem] border-2 border-dashed border-[var(--dash-ink)] bg-white/95 px-6 py-5 text-center shadow-lg">
            <p className="text-sm font-semibold text-[var(--dash-ink)]">Déposez votre PDF ici</p>
            <p className="mt-1 text-[11px] text-neutral-600">ScolIA l’ajoutera à la conversation</p>
          </div>
        </div>
      ) : null}

      {!embedded ? (
        <div className="relative flex items-center justify-between gap-3 border-b border-black/5 bg-white/85 px-5 py-3.5 pt-[max(14px,env(safe-area-inset-top))] backdrop-blur-md">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-[var(--dash-ink)]">
              <ScoliaAiMark size="sm" inverted fill />
            </div>
            <div className="min-w-0">
              <p className="truncate text-sm font-bold text-[var(--dash-ink)]">{SCOLIA_AI_NAME}</p>
              <p className="truncate text-[11px] text-neutral-500">Assistant de votre établissement</p>
            </div>
          </div>

          <div className="flex shrink-0 items-center gap-2">
            <button
              type="button"
              onClick={resetConversation}
              className="rounded-2xl border border-black/8 bg-white px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-ink)] transition hover:bg-[color:var(--dash-soft-muted)]"
            >
              Nouvelle conversation
            </button>
            <button
              type="button"
              onClick={closeChat}
              className="flex h-9 w-9 items-center justify-center rounded-2xl border border-black/8 bg-white text-lg leading-none text-neutral-600 transition hover:bg-[color:var(--dash-soft-muted)] hover:text-[var(--dash-ink)]"
              aria-label={`Fermer ${SCOLIA_AI_NAME}`}
              title="Fermer"
            >
              ×
            </button>
          </div>
        </div>
      ) : null}

      <div
        ref={messagesRef}
        className="relative min-h-0 flex-1 space-y-4 overflow-y-auto bg-gradient-to-b from-[#f4f5f3] via-white to-white px-4 py-5 sm:px-7"
      >
        <div className="mx-auto w-full max-w-2xl space-y-4">
          {messages.length <= 1 ? (
            <div className="flex flex-col items-center justify-center px-2 py-10 text-center sm:py-16">
              <div className="flex h-16 w-16 items-center justify-center overflow-hidden rounded-[1.35rem] bg-[var(--dash-ink)] shadow-sm ring-4 ring-[color:var(--dash-lime)]/30">
                <ScoliaAiMark size="lg" inverted fill />
              </div>
              <p className="mt-5 max-w-sm text-[15px] font-medium leading-relaxed text-[var(--dash-ink)]">
                Posez une question, dictez au micro, ou glissez un PDF.
              </p>
              <p className="mt-2 max-w-sm text-xs leading-relaxed text-neutral-500">
                L’historique reste ici. Les actions importantes vous seront toujours confirmées.
              </p>
            </div>
          ) : null}
          {messages.map((m, i) => {
            if (messages.length <= 1 && m.role === "assistant") return null;
            const isUser = m.role === "user";
            return (
              <div
                key={i}
                className={`flex gap-2.5 ${isUser ? "justify-end" : "justify-start"}`}
              >
                {!isUser ? (
                  <div className="mt-1 flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-[var(--dash-ink)]">
                    <ScoliaAiMark size="sm" inverted fill />
                  </div>
                ) : null}
                <div
                  className={`max-w-[min(92%,36rem)] px-4 py-3 text-[15px] leading-relaxed whitespace-pre-wrap ${
                    isUser
                      ? "rounded-[1.35rem] rounded-br-md bg-[var(--dash-ink)] text-white shadow-sm"
                      : "rounded-[1.35rem] rounded-bl-md border border-black/6 bg-white text-[var(--dash-ink)] shadow-[0_1px_0_rgba(0,0,0,0.03)]"
                  }`}
                >
                  {renderMessageContent(m.content, isUser)}
                </div>
              </div>
            );
          })}
          {renderExtras()}
        </div>
      </div>

      {/* Composer */}
      <div className="border-t border-black/5 bg-white/90 px-4 py-3 pb-[max(16px,env(safe-area-inset-bottom))] backdrop-blur-md sm:px-7">
        <div className="mx-auto w-full max-w-2xl">
          {listening ? (
            <div className="mb-2 flex items-center gap-3 rounded-2xl border border-rose-200/70 bg-rose-50/90 px-3 py-2">
              <span className="relative flex h-8 w-8 items-center justify-center">
                <span className="absolute inset-0 animate-ping rounded-full bg-rose-400/30" />
                <span className="absolute inset-1 animate-pulse rounded-full bg-rose-400/40" />
                <span className="relative h-3 w-3 rounded-full bg-rose-500" />
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-[11px] font-semibold text-rose-900">Écoute en cours…</p>
                <p className="truncate text-[11px] text-rose-800/80">
                  {interimSpeech || "Parlez, je vous écoute"}
                </p>
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
                  className="inline-flex items-center gap-1.5 rounded-full border border-black/8 bg-[color:var(--dash-soft-muted)] px-2.5 py-1 text-[11px] font-medium text-[var(--dash-ink)]"
                >
                  PDF · {f.name}
                  <button
                    type="button"
                    className="text-neutral-400 hover:text-[var(--dash-ink)]"
                    onClick={() => setPendingFiles((prev) => prev.filter((_, i) => i !== idx))}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
          ) : null}

          <div className="flex items-end gap-2 rounded-[1.35rem] border border-black/8 bg-[#f7f8f6] px-2.5 py-2 shadow-[inset_0_1px_0_rgba(255,255,255,0.8)]">
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
              className="shrink-0 rounded-xl px-2.5 py-2 text-sm text-neutral-600 transition hover:bg-white disabled:opacity-40"
            >
              📎
            </button>
            <textarea
              value={input}
              rows={1}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  void send();
                }
              }}
              placeholder={listening ? "Parlez… (envoi auto à la fin)" : "Message à ScolIA…"}
              className="max-h-32 flex-1 resize-none overflow-y-auto bg-transparent px-1 py-2 text-base text-[var(--dash-ink)] placeholder:text-neutral-400 focus:outline-none sm:text-sm"
            />
            <button
              type="button"
              onClick={startVoiceInput}
              disabled={!speechSupported || loading}
              title={speechSupported ? (listening ? "Arrêter et envoyer" : "Dicter (envoi auto)") : "Dictée non supportée"}
              className={`shrink-0 rounded-xl px-2.5 py-2 text-sm font-bold disabled:opacity-40 ${
                listening ? "bg-rose-500 text-white" : "text-neutral-700 hover:bg-white"
              }`}
            >
              🎤
            </button>
            <button
              type="button"
              onClick={() => void send()}
              disabled={loading || (!input.trim() && pendingFiles.length === 0)}
              className="shrink-0 rounded-2xl bg-[var(--dash-ink)] px-3.5 py-2 text-sm font-semibold text-white transition hover:opacity-90 disabled:opacity-50"
            >
              {loading ? "…" : "Envoyer"}
            </button>
          </div>
          {embedded ? (
            <div className="mt-2 flex justify-end">
              <button
                type="button"
                onClick={resetConversation}
                className="text-[11px] font-medium text-neutral-500 transition hover:text-[var(--dash-ink)]"
              >
                Nouvelle conversation
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );

  function renderExtras() {
    return (
      <>
        {pendingChoices ? (
          <div
            className={`rounded-2xl border border-indigo-300/70 bg-indigo-50/90 p-3 space-y-2 ${
              isExpanded ? "" : "mr-4"
            }`}
          >
            {(() => {
              const stepMatch = pendingChoices.promptFr.match(/^Étape\s+(\d+)\s*\/\s*(\d+)\s*[—–-]\s*(.*)$/s);
              if (stepMatch) {
                const [, cur, total, rest] = stepMatch;
                return (
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="inline-flex items-center rounded-full bg-indigo-700 px-2 py-0.5 text-[10px] font-black uppercase tracking-wide text-white">
                        Étape {cur}/{total}
                      </span>
                      <span className="text-[10px] font-semibold text-indigo-800/70">Assistant guidé</span>
                    </div>
                    <p className="text-[12px] font-semibold text-indigo-950 leading-snug">{rest}</p>
                  </div>
                );
              }
              return <p className="text-[11px] font-semibold text-indigo-950">{pendingChoices.promptFr}</p>;
            })()}
            {pendingChoices.selectionType === "date" ? (
              <div className="flex flex-wrap items-center gap-2">
                <input
                  type="date"
                  value={choiceDraft}
                  disabled={loading}
                  onChange={(e) => setChoiceDraft(e.target.value)}
                  className="rounded-lg border border-indigo-200 bg-white px-2.5 py-1.5 text-sm text-slate-800"
                />
                <button
                  type="button"
                  disabled={loading || !choiceDraft}
                  onClick={() => submitChoice(choiceDraft)}
                  className="text-xs rounded-lg bg-indigo-700 text-white px-2.5 py-1.5 font-semibold hover:bg-indigo-800 disabled:opacity-50"
                >
                  Valider
                </button>
              </div>
            ) : pendingChoices.selectionType === "text" ? (
              <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                <input
                  type="text"
                  value={choiceDraft}
                  disabled={loading}
                  placeholder="Saisir le texte…"
                  onChange={(e) => setChoiceDraft(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && choiceDraft.trim()) submitChoice(choiceDraft.trim());
                  }}
                  className="min-w-0 flex-1 rounded-lg border border-indigo-200 bg-white px-2.5 py-1.5 text-sm text-slate-800"
                />
                <button
                  type="button"
                  disabled={loading || !choiceDraft.trim()}
                  onClick={() => submitChoice(choiceDraft.trim())}
                  className="text-xs rounded-lg bg-indigo-700 text-white px-2.5 py-1.5 font-semibold hover:bg-indigo-800 disabled:opacity-50"
                >
                  Valider
                </button>
              </div>
            ) : pendingChoices.selectionType === "multi" ? (
              <div className="space-y-2">
                <div className="flex flex-wrap gap-1.5 max-h-40 overflow-y-auto">
                  {pendingChoices.options.map((o) => {
                    const on = choiceMulti.includes(o.value);
                    return (
                      <button
                        key={o.value}
                        type="button"
                        disabled={loading}
                        onClick={() =>
                          setChoiceMulti((prev) =>
                            on ? prev.filter((v) => v !== o.value) : [...prev, o.value],
                          )
                        }
                        className={`text-[11px] rounded-lg px-2.5 py-1.5 font-semibold border transition ${
                          on
                            ? "bg-indigo-700 text-white border-indigo-700"
                            : "bg-white text-slate-700 border-indigo-200 hover:bg-indigo-100"
                        }`}
                      >
                        {o.label}
                      </button>
                    );
                  })}
                </div>
                <button
                  type="button"
                  disabled={loading || choiceMulti.length === 0}
                  onClick={() => submitChoice(undefined, choiceMulti)}
                  className="text-xs rounded-lg bg-indigo-700 text-white px-2.5 py-1.5 font-semibold hover:bg-indigo-800 disabled:opacity-50"
                >
                  Valider ({choiceMulti.length})
                </button>
              </div>
            ) : pendingChoices.options.length > 0 && pendingChoices.options.length <= 12 ? (
              <div className="flex flex-wrap gap-1.5 max-h-48 overflow-y-auto">
                {pendingChoices.options.map((o) => (
                  <button
                    key={o.value}
                    type="button"
                    disabled={loading}
                    onClick={() => submitChoice(o.value)}
                    className="text-[11px] rounded-lg px-2.5 py-1.5 font-semibold border border-indigo-200 bg-white text-slate-700 hover:bg-indigo-700 hover:text-white hover:border-indigo-700 transition disabled:opacity-50"
                  >
                    {o.label}
                  </button>
                ))}
              </div>
            ) : (
              <div className="flex flex-wrap items-center gap-2">
                <select
                  value={choiceDraft}
                  disabled={loading}
                  onChange={(e) => {
                    const v = e.target.value;
                    setChoiceDraft(v);
                    if (v) submitChoice(v);
                  }}
                  className="min-w-[12rem] flex-1 rounded-lg border border-indigo-200 bg-white px-2.5 py-1.5 text-sm text-slate-800"
                >
                  <option value="">— Choisir —</option>
                  {pendingChoices.options.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </select>
              </div>
            )}
            <button
              type="button"
              disabled={loading}
              onClick={cancelPending}
              className="text-[11px] text-slate-600 underline disabled:opacity-50"
            >
              Annuler
            </button>
          </div>
        ) : null}
        {pendingFileUpload ? (
          <div className="rounded-2xl border-2 border-dashed border-indigo-300 bg-indigo-50/80 px-3 py-3 space-y-2">
            <p className="text-[12px] font-semibold text-indigo-950">{pendingFileUpload.promptFr}</p>
            {pendingFiles.length > 0 ? (
              <p className="text-[11px] text-indigo-800">
                Prêt : {pendingFiles.map((f) => f.name).join(", ")}
              </p>
            ) : (
              <button
                type="button"
                onClick={() => fileInputRef.current?.click()}
                className="w-full rounded-xl border border-indigo-200 bg-white px-3 py-4 text-sm font-medium text-indigo-900 hover:bg-indigo-50"
              >
                Glisser un PDF ici ou cliquer pour choisir
              </button>
            )}
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                disabled={loading || pendingFiles.length === 0}
                onClick={() => {
                  if (!pendingFileUpload) return;
                  const draft = pendingFileUpload.draftArgs;
                  setPendingFileUpload(null);
                  void send({
                    fileApply: { tool: pendingFileUpload.tool, draftArgs: draft },
                    files: pendingFiles,
                  });
                }}
                className="rounded-lg bg-indigo-700 px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-40"
              >
                Continuer avec le PDF
              </button>
              {pendingFileUpload.optional ? (
                <button
                  type="button"
                  disabled={loading}
                  onClick={() => {
                    if (!pendingFileUpload) return;
                    const draft = pendingFileUpload.draftArgs;
                    setPendingFileUpload(null);
                    void send({
                      fileApply: {
                        tool: pendingFileUpload.tool,
                        draftArgs: draft,
                        skipPdf: true,
                      },
                    });
                  }}
                  className="rounded-lg border border-indigo-200 bg-white px-3 py-1.5 text-xs font-semibold text-indigo-900"
                >
                  Sans PDF
                </button>
              ) : null}
            </div>
          </div>
        ) : null}
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
          <div className={`flex flex-wrap gap-2 ${isExpanded ? "" : "mr-4"}`}>
            {ctas.map((c) => {
              if (c.preview) {
                return (
                  <button
                    key={`${c.href}_${c.label}_preview`}
                    type="button"
                    onClick={() => {
                      openUrlPreview(c.href, c.label);
                    }}
                    className="rounded-2xl bg-[color:var(--dash-lime)] px-3 py-2 text-[12px] font-semibold text-[var(--dash-ink)] shadow-sm ring-1 ring-black/5 transition hover:brightness-95"
                  >
                    {c.label}
                  </button>
                );
              }
              const dossierMatch = c.href.match(/^\/eleves\/dossier\/([^/?#]+)\/?$/);
              if (dossierMatch && dossierModal) {
                return (
                  <button
                    key={`${c.href}_${c.label}_dossier`}
                    type="button"
                    onClick={() => {
                      dossierModal.open(dossierMatch[1]!);
                    }}
                    className="rounded-2xl bg-[var(--dash-ink)] px-3 py-2 text-[12px] font-semibold text-white shadow-sm transition hover:opacity-90"
                  >
                    {c.label}
                  </button>
                );
              }
              return (
                <Link
                  key={`${c.href}_${c.label}`}
                  href={c.href}
                  onClick={() => {
                    if (!pageMode) setOpen(false);
                  }}
                  className="rounded-2xl bg-[var(--dash-ink)] px-3 py-2 text-[12px] font-semibold text-white shadow-sm transition hover:opacity-90"
                >
                  {c.label}
                </Link>
              );
            })}
          </div>
        ) : null}
        {loading ? (
          <div
            className={`rounded-2xl border border-black/6 bg-white px-3 py-2.5 text-sm shadow-sm ${
              isExpanded ? "w-fit" : "mr-8"
            }`}
          >
            <div className="inline-flex items-center gap-1.5">
              <span className="h-2 w-2 rounded-full bg-[var(--dash-ink)] animate-bounce [animation-delay:-0.2s]" />
              <span className="h-2 w-2 rounded-full bg-[var(--dash-ink)] animate-bounce [animation-delay:-0.1s]" />
              <span className="h-2 w-2 rounded-full bg-[color:var(--dash-lime)] animate-bounce" />
            </div>
          </div>
        ) : null}
      </>
    );
  }

  if (pageMode) {
    return (
      <>
        <div
          className={
            embedded
              ? "flex h-full min-h-0 w-full flex-col bg-transparent"
              : "min-h-[100dvh] w-full bg-[radial-gradient(ellipse_at_top,_#eef2ff_0%,_#f8fafc_50%,_#f1f5f9_100%)]"
          }
        >
          <div
            className={
              embedded
                ? "flex h-full min-h-0 w-full flex-col"
                : "mx-auto flex h-[100dvh] w-full max-w-4xl flex-col"
            }
          >
            {renderChatBody()}
          </div>
        </div>
        {urlModal ? (
          <ScoliaUrlModal href={urlModal.href} title={urlModal.title} onClose={closeUrlModal} />
        ) : null}
      </>
    );
  }

  if (!mounted || !open) return null;

  return (
    <>
      <div className="fixed inset-0 z-[120] flex items-center justify-center p-3 sm:p-6">
        <button
          type="button"
          className="absolute inset-0 bg-slate-900/45 backdrop-blur-[2px]"
          aria-label="Fermer ScolIA"
          onClick={() => setOpen(false)}
        />
        <div
          ref={panelRef}
          role="dialog"
          aria-modal="true"
          aria-label={SCOLIA_AI_NAME}
          className="relative z-[1] flex h-[min(88dvh,820px)] w-full max-w-3xl flex-col overflow-hidden rounded-3xl border border-slate-200/80 bg-white shadow-[0_32px_80px_-28px_rgba(15,23,42,0.45)]"
        >
          {renderChatBody()}
        </div>
      </div>
      {urlModal ? (
        <ScoliaUrlModal href={urlModal.href} title={urlModal.title} onClose={closeUrlModal} />
      ) : null}
    </>
  );
}
