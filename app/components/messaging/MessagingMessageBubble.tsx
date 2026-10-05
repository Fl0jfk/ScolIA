"use client";

import { useMemo, useState } from "react";
import type { MessagingMessageDto, MessagingReceiptStatus } from "@/app/lib/messaging/types";
import { REACTION_EMOJIS, extractUrls } from "@/app/lib/messaging/constants";
import {
  IconPencil,
  IconTrash,
  IconReply,
  IconForward,
  IconSmile,
  IconCheck,
  IconChecks,
} from "./MessagingIcons";

type Props = {
  message: MessagingMessageDto;
  isMine: boolean;
  onReply: (message: MessagingMessageDto) => void;
  onForward: (message: MessagingMessageDto) => void;
  onEdit: (message: MessagingMessageDto) => void;
  onDelete: (message: MessagingMessageDto) => void;
  onReact: (message: MessagingMessageDto, emoji: string) => void;
};

function formatBytes(n: number): string {
  if (n < 1024) return `${n} o`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} Ko`;
  return `${(n / (1024 * 1024)).toFixed(1)} Mo`;
}

function ReceiptTicks({ status }: { status: MessagingReceiptStatus }) {
  const label =
    status === "read" ? "Lu" : status === "delivered" ? "Distribué" : "Envoyé";
  const color =
    status === "read"
      ? "text-[var(--dash-ink)]"
      : status === "delivered"
        ? "text-[var(--dash-mid)]"
        : "text-neutral-400";
  return (
    <span className={`inline-flex items-center gap-0.5 ${color}`} title={label} aria-label={label}>
      {status === "sent" ? (
        <IconCheck className="h-3 w-3" />
      ) : (
        <IconChecks className="h-3.5 w-3.5" />
      )}
    </span>
  );
}

export default function MessagingMessageBubble({
  message,
  isMine,
  onReply,
  onForward,
  onEdit,
  onDelete,
  onReact,
}: Props) {
  const [showReactions, setShowReactions] = useState(false);
  const deleted = Boolean(message.deletedAt);
  const urls = useMemo(
    () => (message.body && !deleted ? extractUrls(message.body) : []),
    [message.body, deleted],
  );

  const reactionGroups = useMemo(() => {
    const map = new Map<string, number>();
    for (const r of message.reactions) {
      map.set(r.emoji, (map.get(r.emoji) ?? 0) + 1);
    }
    return [...map.entries()];
  }, [message.reactions]);

  return (
    <div className={`group flex flex-col gap-1 ${isMine ? "items-end" : "items-start"}`}>
      {message.replyPreview ? (
        <div
          className={`max-w-[85%] rounded-2xl border-l-2 px-2.5 py-1 text-[11px] text-[var(--dash-mid)] ${
            isMine
              ? "border-[var(--dash-ink)]/40 bg-[var(--dash-ink)]/5"
              : "border-black/15 bg-black/[0.03]"
          }`}
        >
          {message.replyPreview}
        </div>
      ) : null}

      <div
        className={`relative max-w-[85%] rounded-[1.25rem] px-3.5 py-2.5 text-sm shadow-[0_1px_0_rgba(0,0,0,0.03)] ${
          isMine
            ? "rounded-br-md bg-[var(--dash-ink)] text-white"
            : "rounded-bl-md border border-black/6 bg-white text-[var(--dash-ink)]"
        }`}
      >
        {deleted ? (
          <p className={`italic ${isMine ? "text-white/70" : "text-neutral-400"}`}>
            Message supprimé
          </p>
        ) : (
          <>
            {message.forwardedFromId ? (
              <p
                className={`mb-1 text-[10px] uppercase tracking-wide ${
                  isMine ? "text-white/60" : "text-neutral-400"
                }`}
              >
                Transféré
              </p>
            ) : null}

            {message.type === "image" || message.attachments.some((a) => a.mime.startsWith("image/"))
              ? message.attachments
                  .filter((a) => a.mime.startsWith("image/"))
                  .map((a) =>
                    a.url ? (
                      <a key={a.id} href={a.url} target="_blank" rel="noreferrer" className="mb-2 block">
                        {/* eslint-disable-next-line @next/next/no-img-element */}
                        <img
                          src={a.url}
                          alt={a.fileName}
                          className="max-h-56 max-w-full rounded-xl object-contain"
                        />
                      </a>
                    ) : null,
                  )
              : null}

            {message.type === "audio" || message.attachments.some((a) => a.mime.startsWith("audio/"))
              ? message.attachments
                  .filter((a) => a.mime.startsWith("audio/"))
                  .map((a) =>
                    a.url ? (
                      <audio key={a.id} controls className="mb-2 max-w-full" src={a.url}>
                        <track kind="captions" />
                      </audio>
                    ) : null,
                  )
              : null}

            {message.type === "video" || message.attachments.some((a) => a.mime.startsWith("video/"))
              ? message.attachments
                  .filter((a) => a.mime.startsWith("video/"))
                  .map((a) =>
                    a.url ? (
                      <video
                        key={a.id}
                        controls
                        className="mb-2 max-h-56 max-w-full rounded-xl"
                        src={a.url}
                      >
                        <track kind="captions" />
                      </video>
                    ) : null,
                  )
              : null}

            {message.attachments
              .filter(
                (a) =>
                  !a.mime.startsWith("image/") &&
                  !a.mime.startsWith("audio/") &&
                  !a.mime.startsWith("video/"),
              )
              .map((a) => (
                <a
                  key={a.id}
                  href={a.url ?? "#"}
                  target="_blank"
                  rel="noreferrer"
                  className={`mb-2 flex items-center gap-2 rounded-xl px-2 py-1.5 text-xs ${
                    isMine ? "bg-white/15" : "bg-black/[0.04]"
                  }`}
                >
                  <span className="truncate font-medium">{a.fileName}</span>
                  <span className={isMine ? "text-white/60" : "text-neutral-400"}>
                    {formatBytes(a.size)}
                  </span>
                </a>
              ))}

            {message.body ? (
              <p className="whitespace-pre-wrap break-words leading-relaxed">{message.body}</p>
            ) : null}

            {urls.map((url) => (
              <a
                key={url}
                href={url}
                target="_blank"
                rel="noreferrer"
                className={`mt-1 block truncate text-xs underline ${
                  isMine ? "text-white/80" : "text-[var(--dash-ink)]"
                }`}
              >
                {url}
              </a>
            ))}

            {message.editedAt ? (
              <span
                className={`mt-1 block text-[10px] ${isMine ? "text-white/55" : "text-neutral-400"}`}
              >
                modifié
              </span>
            ) : null}
          </>
        )}
      </div>

      {reactionGroups.length > 0 ? (
        <div className="flex flex-wrap gap-1">
          {reactionGroups.map(([emoji, count]) => (
            <button
              key={emoji}
              type="button"
              onClick={() => onReact(message, emoji)}
              className="rounded-full border border-black/6 bg-white px-1.5 py-0.5 text-xs shadow-[0_1px_0_rgba(0,0,0,0.03)]"
            >
              {emoji} {count > 1 ? count : ""}
            </button>
          ))}
        </div>
      ) : null}

      {!deleted ? (
        <div
          className={`flex items-center gap-0.5 opacity-0 transition group-hover:opacity-100 ${
            isMine ? "flex-row-reverse" : ""
          }`}
        >
          <button
            type="button"
            title="Réagir"
            className="rounded-lg p-1 text-neutral-400 hover:bg-black/[0.04] hover:text-[var(--dash-ink)]"
            onClick={() => setShowReactions((v) => !v)}
          >
            <IconSmile className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            title="Répondre"
            className="rounded-lg p-1 text-neutral-400 hover:bg-black/[0.04] hover:text-[var(--dash-ink)]"
            onClick={() => onReply(message)}
          >
            <IconReply className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            title="Transférer"
            className="rounded-lg p-1 text-neutral-400 hover:bg-black/[0.04] hover:text-[var(--dash-ink)]"
            onClick={() => onForward(message)}
          >
            <IconForward className="h-3.5 w-3.5" />
          </button>
          {isMine && message.type === "text" ? (
            <button
              type="button"
              title="Modifier"
              className="rounded-lg p-1 text-neutral-400 hover:bg-black/[0.04] hover:text-[var(--dash-ink)]"
              onClick={() => onEdit(message)}
            >
              <IconPencil className="h-3.5 w-3.5" />
            </button>
          ) : null}
          {isMine ? (
            <button
              type="button"
              title="Supprimer"
              className="rounded-lg p-1 text-neutral-400 hover:bg-rose-50 hover:text-rose-600"
              onClick={() => onDelete(message)}
            >
              <IconTrash className="h-3.5 w-3.5" />
            </button>
          ) : null}
        </div>
      ) : null}

      {showReactions ? (
        <div className="flex gap-1 rounded-full border border-black/6 bg-white px-2 py-1 shadow-[0_8px_24px_-16px_rgba(0,0,0,0.35)]">
          {REACTION_EMOJIS.map((emoji) => (
            <button
              key={emoji}
              type="button"
              className="text-base transition hover:scale-125"
              onClick={() => {
                onReact(message, emoji);
                setShowReactions(false);
              }}
            >
              {emoji}
            </button>
          ))}
        </div>
      ) : null}

      <div
        className={`flex items-center gap-1 text-[10px] text-neutral-400 ${
          isMine ? "flex-row-reverse" : ""
        }`}
      >
        <time>
          {new Date(message.createdAt).toLocaleTimeString("fr-FR", {
            hour: "2-digit",
            minute: "2-digit",
          })}
        </time>
        {isMine && message.receiptStatus && !deleted ? (
          <ReceiptTicks status={message.receiptStatus} />
        ) : null}
      </div>
    </div>
  );
}
