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
}: Props) {
  const [text, setText] = useState("");
  const [pending, setPending] = useState<PendingFile[]>([]);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recording, setRecording] = useState<"audio" | "video" | null>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const typingTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const videoInputRef = useRef<HTMLInputElement | null>(null);
  const videoPreviewRef = useRef<HTMLVideoElement | null>(null);

  useEffect(() => {
    if (editing) setText(editing.body ?? "");
  }, [editing]);

  useEffect(() => {
    return () => {
      for (const p of pending) {
        if (p.previewUrl) URL.revokeObjectURL(p.previewUrl);
      }
      streamRef.current?.getTracks().forEach((t) => t.stop());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- cleanup on unmount only
  }, []);

  const notifyTyping = useCallback(() => {
    if (typingTimer.current) clearTimeout(typingTimer.current);
    typingTimer.current = setTimeout(() => onTyping(), 400);
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

      await sendWithFiles(
        pending.map((p) => p.file),
        body || null,
      );

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

      {recording === "video" ? (
        <div className="mb-2 overflow-hidden rounded-xl bg-slate-900">
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
        <p className="mb-2 rounded-lg bg-red-50 px-2 py-1.5 text-center text-xs font-medium text-red-700">
          Enregistrement vocal… recliquez sur ■ pour envoyer
        </p>
      ) : null}

      {pending.length > 0 ? (
        <div className="mb-2 flex flex-wrap gap-2">
          {pending.map((p, i) => (
            <div key={`${p.file.name}-${i}`} className="relative">
              {p.previewUrl && p.file.type.startsWith("image/") ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={p.previewUrl} alt="" className="h-14 w-14 rounded object-cover" />
              ) : p.previewUrl && p.file.type.startsWith("video/") ? (
                <video src={p.previewUrl} className="h-14 w-14 rounded object-cover" muted />
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
          accept="video/mp4,video/webm,video/quicktime,.mp4,.webm,.mov"
          className="hidden"
          onChange={(e) => {
            if (e.target.files) addFiles(e.target.files);
            e.target.value = "";
          }}
        />

        <button
          type="button"
          title="Pièce jointe"
          disabled={Boolean(editing) || disabled || Boolean(recording)}
          className="rounded-lg p-2 text-slate-500 hover:bg-slate-100 disabled:opacity-40"
          onClick={() => fileInputRef.current?.click()}
        >
          <IconPaperclip className="h-4 w-4" />
        </button>
        <button
          type="button"
          title={recording === "video" ? "Arrêter et envoyer" : "Vidéo (enregistrer ou fichier)"}
          disabled={Boolean(editing) || disabled || recording === "audio"}
          className={`rounded-lg p-2 hover:bg-slate-100 disabled:opacity-40 ${
            recording === "video" ? "text-red-600" : "text-slate-500"
          }`}
          onClick={() => {
            if (recording === "video") {
              stopRecording();
              return;
            }
            // Clic court → enregistrement caméra ; Alt+clic → choisir un fichier
            void startRecording("video");
          }}
          onContextMenu={(e) => {
            e.preventDefault();
            if (!recording && !editing && !disabled) videoInputRef.current?.click();
          }}
        >
          {recording === "video" ? (
            <IconSquare className="h-4 w-4" />
          ) : (
            <IconVideo className="h-4 w-4" />
          )}
        </button>
        <button
          type="button"
          title={recording === "audio" ? "Arrêter et envoyer" : "Message vocal"}
          disabled={Boolean(editing) || disabled || recording === "video"}
          className={`rounded-lg p-2 hover:bg-slate-100 disabled:opacity-40 ${
            recording === "audio" ? "text-red-600" : "text-slate-500"
          }`}
          onClick={() =>
            recording === "audio" ? stopRecording() : void startRecording("audio")
          }
        >
          {recording === "audio" ? (
            <IconSquare className="h-4 w-4" />
          ) : (
            <IconMic className="h-4 w-4" />
          )}
        </button>

        <textarea
          value={text}
          rows={1}
          disabled={disabled || sending || Boolean(recording)}
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
          disabled={
            sending ||
            disabled ||
            Boolean(recording) ||
            (!text.trim() && pending.length === 0 && !editing)
          }
          className="rounded-xl bg-sky-600 p-2 text-white hover:bg-sky-700 disabled:opacity-40"
          onClick={() => void submit()}
        >
          <IconSend className="h-4 w-4" />
        </button>
      </div>
      <p className="mt-1 text-[10px] text-slate-400">
        Vocal / vidéo : clic pour enregistrer, reclic pour envoyer. Clic droit sur vidéo = fichier.
      </p>
    </div>
  );
}
