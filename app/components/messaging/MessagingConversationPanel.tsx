"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { IconX, IconMinus, IconUsers } from "./MessagingIcons";
import type {
  MessagingConversationDto,
  MessagingConversationKind,
  MessagingMessageDto,
  MessagingPeer,
} from "@/app/lib/messaging/types";
import MessagingMessageBubble from "./MessagingMessageBubble";
import MessagingComposer from "./MessagingComposer";

type Props = {
  conversationId: string;
  title: string;
  peer: MessagingPeer | null;
  kind?: MessagingConversationKind;
  membersPreview?: MessagingPeer[];
  memberCount?: number;
  currentUserId: string;
  variant: "dock" | "page" | "mobile-full";
  onClose?: () => void;
  onMinimize?: () => void;
  onForwardRequest?: (message: MessagingMessageDto) => void;
  className?: string;
};

export default function MessagingConversationPanel({
  conversationId,
  title,
  peer,
  kind = "dm",
  membersPreview = [],
  memberCount,
  currentUserId,
  variant,
  onClose,
  onMinimize,
  onForwardRequest,
  className = "",
}: Props) {
  const [messages, setMessages] = useState<MessagingMessageDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [replyTo, setReplyTo] = useState<MessagingMessageDto | null>(null);
  const [editing, setEditing] = useState<MessagingMessageDto | null>(null);
  const [typing, setTyping] = useState(false);
  const bottomRef = useRef<HTMLDivElement | null>(null);
  const typingHideRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const loadMessages = useCallback(async () => {
    const res = await fetch(
      `/api/messaging/conversations/${conversationId}/messages?limit=80`,
      { cache: "no-store" },
    );
    if (!res.ok) return;
    const data = (await res.json()) as { messages?: MessagingMessageDto[] };
    setMessages(Array.isArray(data.messages) ? data.messages : []);
    setLoading(false);
  }, [conversationId]);

  const markRead = useCallback(async () => {
    await fetch(`/api/messaging/conversations/${conversationId}/read`, {
      method: "POST",
    }).catch(() => undefined);
  }, [conversationId]);

  useEffect(() => {
    setLoading(true);
    void loadMessages().then(() => markRead());
  }, [loadMessages, markRead]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, typing]);

  const appendOrReplace = useCallback((msg: MessagingMessageDto) => {
    setMessages((prev) => {
      const idx = prev.findIndex((m) => m.id === msg.id);
      if (idx >= 0) {
        const copy = [...prev];
        copy[idx] = msg;
        return copy;
      }
      return [...prev, msg];
    });
  }, []);

  useEffect(() => {
    const handler = (ev: Event) => {
      const detail = (ev as CustomEvent).detail as {
        conversationId?: string;
        type?: string;
        message?: MessagingMessageDto;
        userId?: string;
      };
      if (detail.conversationId !== conversationId) return;
      if (detail.type === "message" && detail.message) {
        appendOrReplace(detail.message);
        void markRead();
      }
      if (detail.type === "typing" && detail.userId && detail.userId !== currentUserId) {
        setTyping(true);
        if (typingHideRef.current) clearTimeout(typingHideRef.current);
        typingHideRef.current = setTimeout(() => setTyping(false), 2500);
      }
      if (detail.type === "reaction" || detail.type === "conversation_updated") {
        void loadMessages();
      }
    };
    window.addEventListener("scolia-messaging-event", handler);
    return () => window.removeEventListener("scolia-messaging-event", handler);
  }, [conversationId, currentUserId, appendOrReplace, markRead, loadMessages]);

  const sendTyping = useCallback(() => {
    void fetch(`/api/messaging/conversations/${conversationId}/typing`, {
      method: "POST",
    });
  }, [conversationId]);

  const onReact = async (message: MessagingMessageDto, emoji: string) => {
    await fetch(`/api/messaging/messages/${message.id}/reactions`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ emoji }),
    });
    void loadMessages();
  };

  const onDelete = async (message: MessagingMessageDto) => {
    if (!window.confirm("Supprimer ce message ?")) return;
    await fetch(`/api/messaging/messages/${message.id}`, { method: "DELETE" });
    void loadMessages();
  };

  const shell =
    variant === "mobile-full"
      ? "fixed inset-0 z-[140] flex flex-col bg-slate-50"
      : variant === "dock"
        ? "flex h-[420px] w-[340px] flex-col overflow-hidden rounded-t-2xl bg-slate-50 shadow-[0_16px_40px_rgba(15,23,42,0.28)] ring-1 ring-slate-200"
        : "flex h-full min-h-0 flex-col overflow-hidden rounded-xl bg-slate-50 ring-1 ring-slate-200";

  const subtitle =
    kind === "group"
      ? `${memberCount && memberCount > 0 ? memberCount : Math.max(membersPreview.length + 1, 2)} membres`
      : null;

  const typingLabel =
    kind === "group" ? "Quelqu’un écrit…" : `${peer?.name ?? "Contact"} écrit…`;

  return (
    <div className={`${shell} ${className}`}>
      <header className="flex items-center gap-2 border-b border-slate-200/80 bg-gradient-to-r from-white to-sky-50/60 px-3 py-2.5">
        {kind === "group" ? (
          <div className="relative flex h-9 w-9 shrink-0 overflow-hidden rounded-full bg-gradient-to-br from-sky-500 to-indigo-600">
            {membersPreview.slice(0, 2).length === 0 ? (
              <span className="flex h-full w-full items-center justify-center text-xs font-bold text-white">
                <IconUsers className="h-4 w-4" />
              </span>
            ) : (
              <div className="absolute inset-0 grid grid-cols-2">
                {membersPreview.slice(0, 2).map((m) => (
                  <div key={m.id} className="overflow-hidden bg-slate-200">
                    {m.imageUrl ? (
                      // eslint-disable-next-line @next/next/no-img-element
                      <img src={m.imageUrl} alt="" className="h-full w-full object-cover" />
                    ) : (
                      <span className="flex h-full w-full items-center justify-center text-[8px] font-bold text-slate-700">
                        {m.name.slice(0, 1).toUpperCase()}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        ) : (
          <div className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-sky-100 text-sm font-semibold text-sky-800">
            {peer?.imageUrl ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img src={peer.imageUrl} alt="" className="h-full w-full object-cover" />
            ) : (
              (peer?.name || title || "?").slice(0, 1).toUpperCase()
            )}
          </div>
        )}
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold text-slate-800">{title}</p>
          {subtitle ? (
            <p className="truncate text-[11px] text-slate-400">{subtitle}</p>
          ) : null}
        </div>
        {onMinimize ? (
          <button type="button" className="rounded-full p-1.5 text-slate-400 hover:bg-slate-100" onClick={onMinimize}>
            <IconMinus className="h-4 w-4" />
          </button>
        ) : null}
        {onClose ? (
          <button type="button" className="rounded-full p-1.5 text-slate-400 hover:bg-slate-100" onClick={onClose}>
            <IconX className="h-4 w-4" />
          </button>
        ) : null}
      </header>

      <div className="flex-1 space-y-3 overflow-y-auto px-3 py-3">
        {loading ? (
          <p className="text-center text-xs text-slate-400">Chargement…</p>
        ) : messages.length === 0 ? (
          <p className="text-center text-xs text-slate-400">Aucun message — dites bonjour !</p>
        ) : (
          messages.map((m) => (
            <MessagingMessageBubble
              key={m.id}
              message={m}
              isMine={m.senderId === currentUserId}
              onReply={setReplyTo}
              onForward={(msg) => onForwardRequest?.(msg)}
              onEdit={setEditing}
              onDelete={(msg) => void onDelete(msg)}
              onReact={(msg, emoji) => void onReact(msg, emoji)}
            />
          ))
        )}
        {typing ? <p className="text-xs italic text-slate-400">{typingLabel}</p> : null}
        <div ref={bottomRef} />
      </div>

      <MessagingComposer
        conversationId={conversationId}
        replyTo={replyTo}
        onClearReply={() => setReplyTo(null)}
        editing={editing}
        onClearEdit={() => setEditing(null)}
        onSent={() => void loadMessages()}
        onTyping={sendTyping}
      />
    </div>
  );
}

export type { MessagingConversationDto };
