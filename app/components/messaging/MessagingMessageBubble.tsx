"use client";

import { useMemo, useState } from "react";
import type { MessagingMessageDto } from "@/app/lib/messaging/types";
import { REACTION_EMOJIS, extractUrls } from "@/app/lib/messaging/constants";
import {
  IconPencil,
  IconTrash,
  IconReply,
  IconForward,
  IconSmile,
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
          className={`max-w-[85%] rounded-md border-l-2 px-2 py-1 text-[11px] text-slate-500 ${
            isMine ? "border-sky-400 bg-sky-50" : "border-slate-300 bg-slate-50"
          }`}
        >
          {message.replyPreview}
        </div>
      ) : null}

      <div
        className={`relative max-w-[85%] rounded-2xl px-3 py-2 text-sm shadow-sm ${
          isMine
            ? "rounded-br-md bg-sky-600 text-white"
            : "rounded-bl-md bg-white text-slate-800 ring-1 ring-slate-200"
        }`}
      >
        {deleted ? (
          <p className={`italic ${isMine ? "text-sky-100" : "text-slate-400"}`}>
            Message supprimé
          </p>
        ) : (
          <>
            {message.forwardedFromId ? (
              <p className={`mb-1 text-[10px] uppercase tracking-wide ${isMine ? "text-sky-100" : "text-slate-400"}`}>
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
                          className="max-h-56 max-w-full rounded-lg object-contain"
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
                      <video key={a.id} controls className="mb-2 max-h-56 max-w-full rounded-lg" src={a.url}>
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
                  className={`mb-2 flex items-center gap-2 rounded-lg px-2 py-1.5 text-xs ${
                    isMine ? "bg-sky-500/40" : "bg-slate-100"
                  }`}
                >
                  <span className="truncate font-medium">{a.fileName}</span>
                  <span className={isMine ? "text-sky-100" : "text-slate-400"}>
                    {formatBytes(a.size)}
                  </span>
                </a>
              ))}

            {message.body ? (
              <p className="whitespace-pre-wrap break-words">{message.body}</p>
            ) : null}

            {urls.map((url) => (
              <a
                key={url}
                href={url}
                target="_blank"
                rel="noreferrer"
                className={`mt-1 block truncate text-xs underline ${
                  isMine ? "text-sky-100" : "text-sky-700"
                }`}
              >
                {url}
              </a>
            ))}

            {message.editedAt ? (
              <span className={`mt-1 block text-[10px] ${isMine ? "text-sky-100" : "text-slate-400"}`}>
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
              className="rounded-full bg-white px-1.5 py-0.5 text-xs shadow ring-1 ring-slate-200"
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
            className="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-700"
            onClick={() => setShowReactions((v) => !v)}
          >
            <IconSmile className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            title="Répondre"
            className="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-700"
            onClick={() => onReply(message)}
          >
            <IconReply className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            title="Transférer"
            className="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-700"
            onClick={() => onForward(message)}
          >
            <IconForward className="h-3.5 w-3.5" />
          </button>
          {isMine && message.type === "text" ? (
            <button
              type="button"
              title="Modifier"
              className="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-700"
              onClick={() => onEdit(message)}
            >
              <IconPencil className="h-3.5 w-3.5" />
            </button>
          ) : null}
          {isMine ? (
            <button
              type="button"
              title="Supprimer"
              className="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-red-600"
              onClick={() => onDelete(message)}
            >
              <IconTrash className="h-3.5 w-3.5" />
            </button>
          ) : null}
        </div>
      ) : null}

      {showReactions ? (
        <div className="flex gap-1 rounded-full bg-white px-2 py-1 shadow ring-1 ring-slate-200">
          {REACTION_EMOJIS.map((emoji) => (
            <button
              key={emoji}
              type="button"
              className="text-base hover:scale-125"
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

      <time className="text-[10px] text-slate-400">
        {new Date(message.createdAt).toLocaleTimeString("fr-FR", {
          hour: "2-digit",
          minute: "2-digit",
        })}
      </time>
    </div>
  );
}
