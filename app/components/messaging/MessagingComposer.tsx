"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { MessagingAttachmentDto, MessagingMessageDto } from "@/app/lib/messaging/types";
import {
  ACCEPT_FILE_INPUT,
  isAllowedMime,
  maxBytesForMime,
  messageTypeFromMime,
} from "@/app/lib/messaging/constants";
import {
  IconPaperclip,
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
};

export default function MessagingComposer({
  conversationId,
  replyTo,
  onClearReply,
  editing,
  onClearEdit,
  onSent,
  onTyping,
  disabled,
}: Props) {
  const [text, setText] = useState("");
  const [pending, setPending] = useState<PendingFile[]>([]);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const typingTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const videoInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (editing) setText(editing.body ?? "");
  }, [editing]);

  useEffect(() => {
    return () => {
      for (const p of pending) {
        if (p.previewUrl) URL.revokeObjectURL(p.previewUrl);
      }
    };
  }, [pending]);

  const notifyTyping = useCallback(() => {
    if (typingTimer.current) clearTimeout(typingTimer.current);
    typingTimer.current = setTimeout(() => onTyping(), 400);
  }, [onTyping]);

  const addFiles = (files: FileList | File[]) => {
    const list = Array.from(files);
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
        previewUrl: mime.startsWith("image/") ? URL.createObjectURL(file) : undefined,
      });
    }
    if (next.length) {
      setPending((prev) => [...prev, ...next]);
      setError(null);
    }
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

  const submit = async () => {
    if (sending || disabled) return;
    const body = text.trim();
    if (!body && pending.length === 0 && !editing) return;

    setSending(true);
    setError(null);
    try {
      if (editing) {
        const res = await fetch(`/api/messaging/messages/${editing.id}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ body }),
        });
        if (!res.ok) throw new Error("Modification impossible");
        onClearEdit();
        setText("");
        onSent();
        return;
      }

      const uploaded: MessagingAttachmentDto[] = [];
      for (const p of pending) {
        uploaded.push(await uploadFile(p.file));
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
          body: body || null,
          type,
          replyToId: replyTo?.id ?? null,
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

      for (const p of pending) {
        if (p.previewUrl) URL.revokeObjectURL(p.previewUrl);
      }
      setPending([]);
      setText("");
      onClearReply();
      onSent();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur d'envoi");
    } finally {
      setSending(false);
    }
  };

  const startRecording = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const recorder = new MediaRecorder(stream);
      chunksRef.current = [];
      recorder.ondataavailable = (ev) => {
        if (ev.data.size > 0) chunksRef.current.push(ev.data);
      };
      recorder.onstop = () => {
        stream.getTracks().forEach((t) => t.stop());
        const blob = new Blob(chunksRef.current, { type: recorder.mimeType || "audio/webm" });
        const file = new File([blob], `vocal-${Date.now()}.webm`, {
          type: blob.type || "audio/webm",
        });
        addFiles([file]);
      };
      mediaRecorderRef.current = recorder;
      recorder.start();
      setRecording(true);
    } catch {
      setError("Micro inaccessible");
    }
  };

  const stopRecording = () => {
    mediaRecorderRef.current?.stop();
    mediaRecorderRef.current = null;
    setRecording(false);
  };

  return (
    <div className="border-t border-slate-200 bg-white p-2">
      {replyTo ? (
        <div className="mb-1 flex items-center justify-between rounded-md bg-slate-50 px-2 py-1 text-xs text-slate-600">
          <span className="truncate">Réponse à : {replyTo.body || "média"}</span>
          <button type="button" onClick={onClearReply} className="p-0.5">
            <IconX className="h-3.5 w-3.5" />
          </button>
        </div>
      ) : null}
      {editing ? (
        <div className="mb-1 flex items-center justify-between rounded-md bg-amber-50 px-2 py-1 text-xs text-amber-800">
          <span>Modification du message</span>
          <button type="button" onClick={onClearEdit} className="p-0.5">
            <IconX className="h-3.5 w-3.5" />
          </button>
        </div>
      ) : null}

      {pending.length > 0 ? (
        <div className="mb-2 flex flex-wrap gap-2">
          {pending.map((p, i) => (
            <div key={`${p.file.name}-${i}`} className="relative">
              {p.previewUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={p.previewUrl} alt="" className="h-14 w-14 rounded object-cover" />
              ) : (
                <div className="flex h-14 max-w-[8rem] items-center rounded bg-slate-100 px-2 text-[10px]">
                  <span className="truncate">{p.file.name}</span>
                </div>
              )}
              <button
                type="button"
                className="absolute -right-1 -top-1 rounded-full bg-slate-800 p-0.5 text-white"
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

      {error ? <p className="mb-1 text-xs text-red-600">{error}</p> : null}

      <div
        className="flex items-end gap-1"
        onDragOver={(e) => e.preventDefault()}
        onDrop={(e) => {
          e.preventDefault();
          if (e.dataTransfer.files?.length) addFiles(e.dataTransfer.files);
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
          accept="video/mp4,video/webm,video/quicktime"
          className="hidden"
          onChange={(e) => {
            if (e.target.files) addFiles(e.target.files);
            e.target.value = "";
          }}
        />

        <button
          type="button"
          title="Pièce jointe"
          disabled={Boolean(editing) || disabled}
          className="rounded-lg p-2 text-slate-500 hover:bg-slate-100 disabled:opacity-40"
          onClick={() => fileInputRef.current?.click()}
        >
          <IconPaperclip className="h-4 w-4" />
        </button>
        <button
          type="button"
          title="Vidéo"
          disabled={Boolean(editing) || disabled}
          className="rounded-lg p-2 text-slate-500 hover:bg-slate-100 disabled:opacity-40"
          onClick={() => videoInputRef.current?.click()}
        >
          <IconVideo className="h-4 w-4" />
        </button>
        <button
          type="button"
          title={recording ? "Arrêter" : "Message vocal"}
          disabled={Boolean(editing) || disabled}
          className={`rounded-lg p-2 hover:bg-slate-100 disabled:opacity-40 ${
            recording ? "text-red-600" : "text-slate-500"
          }`}
          onClick={() => (recording ? stopRecording() : void startRecording())}
        >
          {recording ? <IconSquare className="h-4 w-4" /> : <IconMic className="h-4 w-4" />}
        </button>

        <textarea
          value={text}
          rows={1}
          disabled={disabled || sending}
          placeholder="Écrire un message…"
          className="max-h-28 min-h-[2.25rem] flex-1 resize-none rounded-xl border border-slate-200 px-3 py-2 text-sm outline-none focus:border-sky-400"
          onChange={(e) => {
            setText(e.target.value);
            notifyTyping();
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void submit();
            }
          }}
        />

        <button
          type="button"
          disabled={sending || disabled || (!text.trim() && pending.length === 0 && !editing)}
          className="rounded-xl bg-sky-600 p-2 text-white hover:bg-sky-700 disabled:opacity-40"
          onClick={() => void submit()}
        >
          <IconSend className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
