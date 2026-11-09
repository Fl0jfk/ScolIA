"use client";

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type {
  MessagingAttachmentDto,
  MessagingMessageDto,
  MessagingPeer,
} from "@/app/lib/messaging/types";
import { messagePreview, normalizeSearch } from "./conversation/utils";
import Avatar from "./conversation/Avatar";
import AssistMenu from "./conversation/AssistMenu";
import {
  ACCEPT_FILE_INPUT,
  isAllowedMime,
  maxBytesForMime,
  messageTypeFromMime,
} from "@/app/lib/messaging/constants";
import {
  IconPaperclip,
  IconSmile,
  IconSend,
  IconMic,
  IconSquare,
  IconVideo,
  IconX,
} from "./MessagingIcons";

type PendingFile = {
  file: File;
  previewUrl?: string;
};

type Props = {
  conversationId: string;
  replyTo: MessagingMessageDto | null;
  onClearReply: () => void;
  editing: MessagingMessageDto | null;
  onClearEdit: () => void;
  onSent: () => void;
  onTyping: () => void;
  disabled?: boolean;
  /** Membres de la conversation (propose les @mentions). */
  members?: MessagingPeer[];
  currentUserId?: string;
  /** ↑ dans un composer vide : modifier mon dernier message. */
  onRequestEditLast?: () => void;
  /** Résumé IA de la conversation (affiché par le panneau). */
  onSummary?: (text: string) => void;
};

const EMOJI_PICKER = [
  "😀", "😂", "🙂", "😉", "😍", "🥰", "😎", "🤔", "😅", "😢", "😮", "😡",
  "👍", "👎", "👏", "🙏", "💪", "🎉", "🔥", "❤️", "✅", "❌", "⭐", "👀",
];

const draftKey = (conversationId: string) => `scolia-msg-draft:${conversationId}`;

function readDraft(conversationId: string): string {
  try {
    return window.localStorage.getItem(draftKey(conversationId)) ?? "";
  } catch {
    return "";
  }
}

function writeDraft(conversationId: string, value: string) {
  try {
    if (value) window.localStorage.setItem(draftKey(conversationId), value);
    else window.localStorage.removeItem(draftKey(conversationId));
  } catch {
    /* stockage indisponible */
  }
}

const MENTION_RE = /(?:^|\s)@([\p{L}\p{M}'’.-]*(?: [\p{L}\p{M}'’.-]*)?)$/u;

function pickRecorderMime(kind: "audio" | "video"): string | undefined {
  if (typeof MediaRecorder === "undefined" || !MediaRecorder.isTypeSupported) {
    return undefined;
  }
  const candidates =
    kind === "audio"
      ? [
          "audio/webm;codecs=opus",
          "audio/webm",
          "audio/mp4",
          "audio/ogg;codecs=opus",
          "audio/ogg",
        ]
      : [
          "video/webm;codecs=vp9,opus",
          "video/webm;codecs=vp8,opus",
          "video/webm",
          "video/mp4",
        ];
  return candidates.find((t) => MediaRecorder.isTypeSupported(t));
}

function extensionForMime(mime: string, fallback: string): string {
  if (mime.includes("mp4")) return "mp4";
  if (mime.includes("ogg")) return "ogg";
  if (mime.includes("wav")) return "wav";
  if (mime.includes("webm")) return "webm";
  return fallback;
}

function normalizeFileMime(file: File): File {
  if (file.type && file.type !== "application/octet-stream") return file;
  const name = file.name.toLowerCase();
  let mime = file.type;
  if (name.endsWith(".mp4") || name.endsWith(".m4v")) mime = "video/mp4";
  else if (name.endsWith(".webm")) mime = name.includes("audio") ? "audio/webm" : "video/webm";
  else if (name.endsWith(".mov")) mime = "video/quicktime";
  else if (name.endsWith(".mp3")) mime = "audio/mpeg";
  else if (name.endsWith(".wav")) mime = "audio/wav";
  else if (name.endsWith(".ogg") || name.endsWith(".oga")) mime = "audio/ogg";
  else if (name.endsWith(".m4a")) mime = "audio/mp4";
  if (!mime || mime === file.type) return file;
  return new File([file], file.name, { type: mime, lastModified: file.lastModified });
}

export default function MessagingComposer({
  conversationId,
  replyTo,
  onClearReply,
  editing,
  onClearEdit,
  onSent,
  onTyping,
  disabled,
  members,
  currentUserId,
  onRequestEditLast,
  onSummary,
}: Props) {
  const [text, setText] = useState("");
  const [mentioned, setMentioned] = useState<Array<{ id: string; name: string }>>([]);
  const [mentionQuery, setMentionQuery] = useState<{ query: string; start: number } | null>(null);
  const [mentionIndex, setMentionIndex] = useState(0);
  const [dragOver, setDragOver] = useState(false);
  const [emojiOpen, setEmojiOpen] = useState(false);
  const [focused, setFocused] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const stashedDraft = useRef("");
  const prevEditing = useRef<MessagingMessageDto | null>(null);
  const lastTypingSent = useRef(0);
  const [pending, setPending] = useState<PendingFile[]>([]);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recording, setRecording] = useState<"audio" | "video" | null>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const videoInputRef = useRef<HTMLInputElement | null>(null);
  const videoPreviewRef = useRef<HTMLVideoElement | null>(null);

  // Brouillon par conversation.
  useEffect(() => {
    setText(readDraft(conversationId));
    setMentioned([]);
    setMentionQuery(null);
    setEmojiOpen(false);
  }, [conversationId]);

  // Entrée / sortie du mode modification : on met le brouillon de côté puis on le restaure.
  useEffect(() => {
    if (editing && !prevEditing.current) {
      stashedDraft.current = text;
      setText(editing.body ?? "");
      setMentionQuery(null);
      requestAnimationFrame(() => textareaRef.current?.focus());
    } else if (editing && prevEditing.current && prevEditing.current.id !== editing.id) {
      setText(editing.body ?? "");
    } else if (!editing && prevEditing.current) {
      setText(readDraft(conversationId));
      stashedDraft.current = "";
    }
    prevEditing.current = editing;
    // eslint-disable-next-line react-hooks/exhaustive-deps -- réagit uniquement à `editing`
  }, [editing]);

  useEffect(() => {
    if (replyTo) textareaRef.current?.focus();
  }, [replyTo]);

  // Zone qui grandit (max ~6 lignes).
  useLayoutEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 144)}px`;
  }, [text]);

  const mentionCandidates = useMemo(() => {
    if (!mentionQuery || !members) return [];
    const q = normalizeSearch(mentionQuery.query);
    return members
      .filter((m) => m.id !== currentUserId && normalizeSearch(m.name).includes(q))
      .slice(0, 6);
  }, [mentionQuery, members, currentUserId]);

  const setDraft = (value: string) => {
    setText(value);
    if (!editing) writeDraft(conversationId, value);
  };

  const updateMentionQuery = (value: string, caret: number) => {
    if (!members || members.length === 0 || editing) {
      setMentionQuery(null);
      return;
    }
    const before = value.slice(0, caret);
    const m = MENTION_RE.exec(before);
    if (!m) {
      setMentionQuery(null);
      return;
    }
    setMentionQuery({ query: m[1], start: before.length - m[1].length - 1 });
    setMentionIndex(0);
  };

  const insertMention = (peer: MessagingPeer) => {
    const el = textareaRef.current;
    if (!mentionQuery || !el) return;
    const caret = el.selectionStart ?? text.length;
    const insert = `@${peer.name} `;
    const next = text.slice(0, mentionQuery.start) + insert + text.slice(caret);
    setDraft(next);
    setMentioned((prev) => (prev.some((p) => p.id === peer.id) ? prev : [...prev, { id: peer.id, name: peer.name }]));
    setMentionQuery(null);
    const pos = mentionQuery.start + insert.length;
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(pos, pos);
    });
  };

  const insertEmoji = (emoji: string) => {
    const el = textareaRef.current;
    const start = el?.selectionStart ?? text.length;
    const end = el?.selectionEnd ?? text.length;
    setDraft(text.slice(0, start) + emoji + text.slice(end));
    setEmojiOpen(false);
    const pos = start + emoji.length;
    requestAnimationFrame(() => {
      el?.focus();
      el?.setSelectionRange(pos, pos);
    });
  };

  // Brouillon proposé par l'IA : remplace le texte, l'humain relit puis envoie.
  const applyAssistDraft = (value: string) => {
    setDraft(value);
    setMentionQuery(null);
    requestAnimationFrame(() => {
      const el = textareaRef.current;
      if (!el) return;
      el.focus();
      el.setSelectionRange(value.length, value.length);
    });
  };

  useEffect(() => {
    return () => {
      for (const p of pending) {
        if (p.previewUrl) URL.revokeObjectURL(p.previewUrl);
      }
      streamRef.current?.getTracks().forEach((t) => t.stop());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- cleanup on unmount only
  }, []);

  // Au plus un signal « écrit… » toutes les 2,5 s tant qu’on tape.
  const notifyTyping = useCallback(() => {
    const now = Date.now();
    if (now - lastTypingSent.current < 2500) return;
    lastTypingSent.current = now;
    onTyping();
  }, [onTyping]);

  const addFiles = (files: FileList | File[]) => {
    const list = Array.from(files).map(normalizeFileMime);
    const next: PendingFile[] = [];
    for (const file of list) {
      const mime = file.type || "application/octet-stream";
      if (!isAllowedMime(mime)) {
        setError(`Format non accepté : ${file.name}`);
        continue;
      }
      if (file.size > maxBytesForMime(mime)) {
        setError(`Fichier trop volumineux : ${file.name}`);
        continue;
      }
      next.push({
        file,
        previewUrl:
          mime.startsWith("image/") || mime.startsWith("video/")
            ? URL.createObjectURL(file)
            : undefined,
      });
    }
    if (next.length) {
      setPending((prev) => [...prev, ...next]);
      setError(null);
    }
    return next;
  };

  const uploadFile = async (file: File): Promise<MessagingAttachmentDto> => {
    const fd = new FormData();
    fd.append("file", file);
    fd.append("conversationId", conversationId);
    const res = await fetch("/api/messaging/upload", { method: "POST", body: fd });
    if (!res.ok) {
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      throw new Error(data.error || "Échec de l'upload");
    }
    return (await res.json()) as MessagingAttachmentDto;
  };

  const sendWithFiles = async (
    files: File[],
    bodyText: string | null = null,
    replyId: string | null = replyTo?.id ?? null,
    mentionUserIds: string[] = [],
  ) => {
    const uploaded: MessagingAttachmentDto[] = [];
    for (const file of files) {
      uploaded.push(await uploadFile(file));
    }
    const primaryMime = uploaded[0]?.mime ?? "";
    const type =
      uploaded.length === 0
        ? "text"
        : uploaded.length === 1
          ? messageTypeFromMime(primaryMime)
          : uploaded.every((a) => a.mime.startsWith("image/"))
            ? "image"
            : "file";

    const res = await fetch(`/api/messaging/conversations/${conversationId}/messages`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        body: bodyText,
        type,
        replyToId: replyId,
        mentionUserIds,
        attachments: uploaded.map((a) => ({
          s3Key: a.s3Key,
          mime: a.mime,
          size: a.size,
          fileName: a.fileName,
          width: a.width,
          height: a.height,
        })),
      }),
    });
    if (!res.ok) {
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      throw new Error(data.error || "Envoi impossible");
    }
  };

  const submit = async () => {
    if (sending || disabled) return;
    const body = text.trim();
    if (!body && pending.length === 0 && !editing) return;

    setSending(true);
    setError(null);
    try {
      if (editing) {
        if (!body) throw new Error("Le message ne peut pas être vide.");
        const res = await fetch(`/api/messaging/messages/${editing.id}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ body }),
        });
        if (!res.ok) throw new Error("Modification impossible");
        onClearEdit();
        onSent();
        return;
      }

      const mentionUserIds = mentioned
        .filter((p) => body.toLowerCase().includes(`@${p.name}`.toLowerCase()))
        .map((p) => p.id);
      await sendWithFiles(
        pending.map((p) => p.file),
        body || null,
        replyTo?.id ?? null,
        mentionUserIds,
      );

      for (const p of pending) {
        if (p.previewUrl) URL.revokeObjectURL(p.previewUrl);
      }
      setPending([]);
      setText("");
      writeDraft(conversationId, "");
      setMentioned([]);
      setMentionQuery(null);
      onClearReply();
      onSent();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur d'envoi");
    } finally {
      setSending(false);
      requestAnimationFrame(() => textareaRef.current?.focus());
    }
  };

  const stopMediaTracks = () => {
    streamRef.current?.getTracks().forEach((t) => t.stop());
    streamRef.current = null;
    if (videoPreviewRef.current) {
      videoPreviewRef.current.srcObject = null;
    }
  };

  const startRecording = async (kind: "audio" | "video") => {
    if (recording || sending || disabled || editing) return;
    if (typeof window === "undefined" || !navigator.mediaDevices?.getUserMedia) {
      setError(
        kind === "audio"
          ? "Micro non disponible dans ce navigateur."
          : "Caméra non disponible dans ce navigateur.",
      );
      return;
    }
    if (typeof MediaRecorder === "undefined") {
      setError("Enregistrement non supporté par ce navigateur.");
      return;
    }

    setError(null);
    try {
      const stream = await navigator.mediaDevices.getUserMedia(
        kind === "audio"
          ? { audio: { echoCancellation: true, noiseSuppression: true } }
          : { audio: true, video: { facingMode: "user", width: { ideal: 720 }, height: { ideal: 1280 } } },
      );
      streamRef.current = stream;
      const mimeType = pickRecorderMime(kind);
      const recorder = mimeType
        ? new MediaRecorder(stream, { mimeType })
        : new MediaRecorder(stream);
      chunksRef.current = [];
      recorder.ondataavailable = (ev) => {
        if (ev.data.size > 0) chunksRef.current.push(ev.data);
      };
      recorder.onerror = () => {
        setError("Erreur d’enregistrement.");
        stopMediaTracks();
        setRecording(null);
      };
      recorder.onstop = () => {
        const blobType = recorder.mimeType || (kind === "audio" ? "audio/webm" : "video/webm");
        const blob = new Blob(chunksRef.current, { type: blobType });
        stopMediaTracks();
        setRecording(null);
        if (blob.size < 256) {
          setError("Enregistrement trop court.");
          return;
        }
        const ext = extensionForMime(blobType, kind === "audio" ? "webm" : "webm");
        const file = new File(
          [blob],
          `${kind === "audio" ? "vocal" : "video"}-${Date.now()}.${ext}`,
          { type: blobType.split(";")[0] || blobType },
        );
        void (async () => {
          setSending(true);
          setError(null);
          try {
            await sendWithFiles([file], null);
            onClearReply();
            onSent();
          } catch (e) {
            setError(e instanceof Error ? e.message : "Envoi impossible");
            addFiles([file]);
          } finally {
            setSending(false);
          }
        })();
      };
      mediaRecorderRef.current = recorder;
      recorder.start(250);
      setRecording(kind);
      if (kind === "video") {
        requestAnimationFrame(() => {
          if (videoPreviewRef.current) {
            videoPreviewRef.current.srcObject = stream;
            void videoPreviewRef.current.play().catch(() => undefined);
          }
        });
      }
    } catch (err) {
      stopMediaTracks();
      const name = err instanceof DOMException ? err.name : "";
      if (name === "NotAllowedError" || name === "PermissionDeniedError") {
        setError(
          kind === "audio"
            ? "Autorisez le micro dans le navigateur (icône cadenas → Micro)."
            : "Autorisez la caméra dans le navigateur (icône cadenas → Caméra).",
        );
      } else if (name === "NotFoundError" || name === "DevicesNotFoundError") {
        setError(kind === "audio" ? "Aucun micro détecté." : "Aucune caméra détectée.");
      } else if (name === "NotReadableError" || name === "TrackStartError") {
        setError(
          kind === "audio"
            ? "Micro déjà utilisé par une autre application."
            : "Caméra déjà utilisée par une autre application.",
        );
      } else {
        setError(kind === "audio" ? "Micro inaccessible." : "Caméra inaccessible.");
      }
    }
  };

  const stopRecording = () => {
    const recorder = mediaRecorderRef.current;
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
    } else {
      stopMediaTracks();
      setRecording(null);
    }
    mediaRecorderRef.current = null;
  };

  return (
    <div className="relative border-t border-black/5 bg-[#eceeea]/40 p-2.5 pb-[max(0.625rem,env(safe-area-inset-bottom))] sm:p-3 sm:pb-[max(0.75rem,env(safe-area-inset-bottom))]">
      {replyTo ? (
        <div className="mb-1.5 flex items-center justify-between gap-2 rounded-2xl border-l-2 border-[var(--dash-ink)]/40 bg-white/80 px-2.5 py-1.5 text-xs text-[var(--dash-mid)]">
          <span className="min-w-0 truncate">
            Réponse à <strong className="font-semibold">{replyTo.senderName}</strong> :{" "}
            {messagePreview(replyTo)}
          </span>
          <button
            type="button"
            onClick={onClearReply}
            aria-label="Annuler la réponse"
            className="rounded p-0.5 outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50"
          >
            <IconX className="h-3.5 w-3.5" />
          </button>
        </div>
      ) : null}
      {editing ? (
        <div className="mb-1.5 flex items-center justify-between rounded-2xl bg-amber-50 px-2.5 py-1.5 text-xs text-amber-900">
          <span>
            Modification du message <span className="text-amber-700">· Échap pour annuler</span>
          </span>
          <button
            type="button"
            onClick={onClearEdit}
            aria-label="Annuler la modification"
            className="rounded p-0.5 outline-none focus-visible:ring-2 focus-visible:ring-amber-700/50"
          >
            <IconX className="h-3.5 w-3.5" />
          </button>
        </div>
      ) : null}

      {recording === "video" ? (
        <div className="mb-2 overflow-hidden rounded-2xl bg-[var(--dash-ink)]">
          <video
            ref={videoPreviewRef}
            muted
            playsInline
            autoPlay
            className="max-h-40 w-full object-cover"
          />
          <p className="px-2 py-1 text-center text-[11px] text-white/80">
            Enregistrement vidéo… recliquez sur ■ pour envoyer
          </p>
        </div>
      ) : null}

      {recording === "audio" ? (
        <p className="mb-2 rounded-2xl bg-rose-50 px-2 py-1.5 text-center text-xs font-medium text-rose-700">
          Enregistrement vocal… recliquez sur ■ pour envoyer
        </p>
      ) : null}

      {pending.length > 0 ? (
        <div className="mb-2 flex flex-wrap gap-2">
          {pending.map((p, i) => (
            <div key={`${p.file.name}-${i}`} className="relative">
              {p.previewUrl && p.file.type.startsWith("image/") ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={p.previewUrl} alt="" className="h-14 w-14 rounded-xl object-cover" />
              ) : p.previewUrl && p.file.type.startsWith("video/") ? (
                <video src={p.previewUrl} className="h-14 w-14 rounded-xl object-cover" muted />
              ) : (
                <div className="flex h-14 max-w-[8rem] items-center rounded-xl bg-white px-2 text-[10px]">
                  <span className="truncate">{p.file.name}</span>
                </div>
              )}
              <button
                type="button"
                aria-label={`Retirer ${p.file.name}`}
                className="absolute -right-1 -top-1 rounded-full bg-[var(--dash-ink)] p-0.5 text-white outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-lime)]"
                onClick={() => {
                  if (p.previewUrl) URL.revokeObjectURL(p.previewUrl);
                  setPending((prev) => prev.filter((_, idx) => idx !== i));
                }}
              >
                <IconX className="h-3 w-3" />
              </button>
            </div>
          ))}
        </div>
      ) : null}

      {error ? (
        <p role="alert" className="mb-1 text-xs text-rose-700">
          {error}
        </p>
      ) : null}

      {mentionCandidates.length > 0 ? (
        <ul
          role="listbox"
          aria-label="Mentionner un membre"
          className="absolute inset-x-3 bottom-full z-20 mb-1 max-h-56 overflow-y-auto rounded-2xl border border-black/8 bg-white p-1 shadow-[0_12px_32px_-12px_rgba(0,0,0,0.4)]"
        >
          {mentionCandidates.map((m, i) => (
            <li key={m.id} role="option" aria-selected={i === mentionIndex}>
              <button
                type="button"
                className={`flex w-full items-center gap-2 rounded-xl px-2 py-1.5 text-left text-sm ${
                  i === mentionIndex ? "bg-black/[0.06]" : "hover:bg-black/[0.04]"
                }`}
                onMouseDown={(e) => {
                  e.preventDefault();
                  insertMention(m);
                }}
              >
                <Avatar name={m.name} imageUrl={m.imageUrl} size={24} />
                <span className="truncate font-medium text-[var(--dash-ink)]">{m.name}</span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}

      {emojiOpen ? (
        <div
          role="dialog"
          aria-label="Choisir un emoji"
          className="absolute bottom-full left-3 z-20 mb-1 grid w-60 grid-cols-8 gap-0.5 rounded-2xl border border-black/8 bg-white p-2 shadow-[0_12px_32px_-12px_rgba(0,0,0,0.4)]"
        >
          {EMOJI_PICKER.map((emoji) => (
            <button
              key={emoji}
              type="button"
              aria-label={emoji}
              className="rounded-lg p-1 text-lg transition hover:bg-black/[0.06]"
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => insertEmoji(emoji)}
            >
              {emoji}
            </button>
          ))}
        </div>
      ) : null}

      <div
        className={`rounded-[1.25rem] border bg-white transition ${
          dragOver ? "border-[var(--dash-ink)] ring-2 ring-[var(--dash-lime)]" : "border-black/6"
        } focus-within:border-[var(--dash-ink)]/40`}
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDragOver(false);
          if (!editing && e.dataTransfer.files?.length) addFiles(e.dataTransfer.files);
        }}
      >
        <input
          ref={fileInputRef}
          type="file"
          multiple
          accept={ACCEPT_FILE_INPUT}
          className="hidden"
          onChange={(e) => {
            if (e.target.files) addFiles(e.target.files);
            e.target.value = "";
          }}
        />
        <input
          ref={videoInputRef}
          type="file"
          accept="video/mp4,video/webm,video/quicktime,.mp4,.webm,.mov"
          className="hidden"
          onChange={(e) => {
            if (e.target.files) addFiles(e.target.files);
            e.target.value = "";
          }}
        />

        <textarea
          ref={textareaRef}
          value={text}
          rows={1}
          aria-label="Écrire un message"
          disabled={disabled || sending || Boolean(recording)}
          placeholder={editing ? "Modifier le message…" : "Écrire un message…"}
          className="max-h-36 min-h-[2.25rem] w-full resize-none border-none bg-transparent px-3 pb-1 pt-2.5 text-sm text-[var(--dash-ink)] outline-none placeholder:text-neutral-500 focus:ring-0"
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          onChange={(e) => {
            setDraft(e.target.value);
            updateMentionQuery(e.target.value, e.target.selectionStart ?? e.target.value.length);
            if (e.target.value) notifyTyping();
          }}
          onClick={(e) =>
            updateMentionQuery(text, e.currentTarget.selectionStart ?? text.length)
          }
          onPaste={(e) => {
            if (editing) return;
            const files = Array.from(e.clipboardData.files ?? []);
            if (files.length > 0) {
              e.preventDefault();
              addFiles(files);
            }
          }}
          onKeyDown={(e) => {
            if (e.nativeEvent.isComposing) return;
            if (mentionCandidates.length > 0) {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setMentionIndex((i) => (i + 1) % mentionCandidates.length);
                return;
              }
              if (e.key === "ArrowUp") {
                e.preventDefault();
                setMentionIndex((i) => (i - 1 + mentionCandidates.length) % mentionCandidates.length);
                return;
              }
              if (e.key === "Enter" || e.key === "Tab") {
                e.preventDefault();
                insertMention(mentionCandidates[mentionIndex] ?? mentionCandidates[0]);
                return;
              }
              if (e.key === "Escape") {
                e.preventDefault();
                setMentionQuery(null);
                return;
              }
            }
            if (e.key === "Escape") {
              if (emojiOpen) {
                setEmojiOpen(false);
              } else if (editing) {
                e.preventDefault();
                onClearEdit();
              } else if (replyTo) {
                e.preventDefault();
                onClearReply();
              }
              return;
            }
            if (e.key === "ArrowUp" && !text && !editing && pending.length === 0) {
              e.preventDefault();
              onRequestEditLast?.();
              return;
            }
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void submit();
            }
          }}
        />

        <div className="flex items-center gap-0.5 px-1.5 pb-1.5">
          <button
            type="button"
            title="Pièce jointe"
            aria-label="Joindre un fichier"
            disabled={Boolean(editing) || disabled || Boolean(recording)}
            className="rounded-lg p-1.5 text-[var(--dash-mid)] outline-none transition hover:bg-black/[0.06] hover:text-[var(--dash-ink)] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 disabled:opacity-40"
            onClick={() => fileInputRef.current?.click()}
          >
            <IconPaperclip className="h-[18px] w-[18px]" />
          </button>
          <button
            type="button"
            title="Emoji"
            aria-label="Insérer un emoji"
            aria-expanded={emojiOpen}
            disabled={disabled || Boolean(recording)}
            className="rounded-lg p-1.5 text-[var(--dash-mid)] outline-none transition hover:bg-black/[0.06] hover:text-[var(--dash-ink)] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 disabled:opacity-40"
            onClick={() => setEmojiOpen((v) => !v)}
          >
            <IconSmile className="h-[18px] w-[18px]" />
          </button>
          <button
            type="button"
            title="Joindre une vidéo"
            aria-label="Joindre une vidéo"
            disabled={Boolean(editing) || disabled || Boolean(recording)}
            className="rounded-lg p-1.5 text-[var(--dash-mid)] outline-none transition hover:bg-black/[0.06] hover:text-[var(--dash-ink)] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 disabled:opacity-40"
            onClick={() => videoInputRef.current?.click()}
          >
            <IconVideo className="h-[18px] w-[18px]" />
          </button>
          <button
            type="button"
            title={recording === "audio" ? "Arrêter et envoyer" : "Message vocal"}
            aria-label={
              recording === "audio" ? "Arrêter et envoyer le vocal" : "Enregistrer un message vocal"
            }
            disabled={Boolean(editing) || disabled || recording === "video"}
            className={`rounded-lg p-1.5 outline-none transition hover:bg-black/[0.06] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 disabled:opacity-40 ${
              recording === "audio" ? "text-rose-600" : "text-[var(--dash-mid)] hover:text-[var(--dash-ink)]"
            }`}
            onClick={() => (recording === "audio" ? stopRecording() : void startRecording("audio"))}
          >
            {recording === "audio" ? (
              <IconSquare className="h-[18px] w-[18px]" />
            ) : (
              <IconMic className="h-[18px] w-[18px]" />
            )}
          </button>
          {onSummary ? (
            <AssistMenu
              conversationId={conversationId}
              draft={text}
              onDraft={applyAssistDraft}
              onSummary={onSummary}
              disabled={disabled || sending || Boolean(recording)}
            />
          ) : null}

          <span
            className={`ml-2 hidden min-w-0 flex-1 truncate text-[11px] text-neutral-500 sm:block ${
              focused ? "visible" : "invisible"
            }`}
            aria-hidden
          >
            Entrée pour envoyer · Maj+Entrée nouvelle ligne
            {members && members.length > 0 ? " · @ pour mentionner" : ""}
          </span>
          <span className="flex-1 sm:hidden" />

          {text.length > 800 ? (
            <span className="mr-1 shrink-0 text-[11px] tabular-nums text-neutral-500">
              {text.length} caractères
            </span>
          ) : null}
          <button
            type="button"
            aria-label={editing ? "Enregistrer la modification" : "Envoyer"}
            title={editing ? "Enregistrer" : "Envoyer"}
            disabled={
              sending ||
              disabled ||
              Boolean(recording) ||
              (!text.trim() && pending.length === 0 && !editing)
            }
            className="shrink-0 rounded-xl bg-[var(--dash-ink)] p-2 text-white outline-none transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-[var(--dash-lime)] focus-visible:ring-offset-1 disabled:opacity-35"
            onClick={() => void submit()}
          >
            <IconSend className="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
