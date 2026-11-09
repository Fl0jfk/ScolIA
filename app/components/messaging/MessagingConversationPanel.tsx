"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  IconArrowDown,
  IconInfo,
  IconMinus,
  IconPin,
  IconUsers,
  IconVideoCall,
  IconX,
} from "./MessagingIcons";
import type {
  MessagingConversationDto,
  MessagingConversationKind,
  MessagingMessageDto,
  MessagingPeer,
  MessagingPresenceStatus,
} from "@/app/lib/messaging/types";
import { MESSAGING_ROOT_ATTR } from "@/app/lib/messaging/dock";
import MessagingMessageBubble from "./MessagingMessageBubble";
import MessagingComposer from "./MessagingComposer";
import MessagingPresenceBadge from "./MessagingPresenceBadge";
import TypingBubble from "./conversation/TypingBubble";
import {
  buildItems,
  buildMembersMap,
  firstName,
  messagePreview,
} from "./conversation/utils";

type Props = {
  conversationId: string;
  title: string;
  peer: MessagingPeer | null;
  kind?: MessagingConversationKind;
  membersPreview?: MessagingPeer[];
  memberCount?: number;
  currentUserId: string;
  presenceStatus?: MessagingPresenceStatus;
  variant: "dock" | "page" | "mobile-full";
  onClose?: () => void;
  onMinimize?: () => void;
  onForwardRequest?: (message: MessagingMessageDto) => void;
  onStartVideoCall?: () => void;
  className?: string;
  /** Membres complets (noms en groupe, @mentions, « Vu par »). */
  members?: MessagingPeer[];
  /** Bouton « i » de l’en-tête (tiroir de détails de la page). */
  onOpenDetails?: () => void;
  /** Messages épinglés connus de la page (complète ceux déjà chargés). */
  pinnedMessages?: MessagingMessageDto[];
};

const BOTTOM_THRESHOLD = 80;

function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
}

export default function MessagingConversationPanel({
  conversationId,
  title,
  peer,
  kind = "dm",
  membersPreview = [],
  memberCount,
  currentUserId,
  presenceStatus = "offline",
  variant,
  onClose,
  onMinimize,
  onForwardRequest,
  onStartVideoCall,
  className = "",
  members,
  onOpenDetails,
  pinnedMessages,
}: Props) {
  const [messages, setMessages] = useState<MessagingMessageDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [replyTo, setReplyTo] = useState<MessagingMessageDto | null>(null);
  const [editing, setEditing] = useState<MessagingMessageDto | null>(null);
  const [typingIds, setTypingIds] = useState<string[]>([]);
  const [confirmDelete, setConfirmDelete] = useState<MessagingMessageDto | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [highlightId, setHighlightId] = useState<string | null>(null);
  const [newCount, setNewCount] = useState(0);
  const [nearBottom, setNearBottom] = useState(true);
  const [pinIndex, setPinIndex] = useState(0);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  const atBottomRef = useRef(true);
  const prevLastIdRef = useRef<string | null>(null);
  const typingTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const toastTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const highlightTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const activeConversation = useRef(conversationId);
  const forwardRef = useRef(onForwardRequest);

  useEffect(() => {
    forwardRef.current = onForwardRequest;
  }, [onForwardRequest]);

  const membersById = useMemo(
    () => buildMembersMap(members, [peer, ...membersPreview]),
    [members, peer, membersPreview],
  );

  const mentionMembers = useMemo(
    () => (members && members.length > 0 ? members : [...membersById.values()]),
    [members, membersById],
  );

  const showToast = useCallback((text: string) => {
    setToast(text);
    if (toastTimer.current) clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToast(null), 1800);
  }, []);

  const loadMessages = useCallback(async () => {
    const id = conversationId;
    try {
      const res = await fetch(`/api/messaging/conversations/${id}/messages?limit=80`, {
        cache: "no-store",
      });
      if (!res.ok || activeConversation.current !== id) return;
      const data = (await res.json()) as { messages?: MessagingMessageDto[] };
      if (activeConversation.current !== id) return;
      setMessages(Array.isArray(data.messages) ? data.messages : []);
    } catch {
      /* réseau : on garde l’état courant */
    } finally {
      if (activeConversation.current === id) setLoading(false);
    }
  }, [conversationId]);

  const markRead = useCallback(async () => {
    await fetch(`/api/messaging/conversations/${conversationId}/read`, {
      method: "POST",
    }).catch(() => undefined);
  }, [conversationId]);

  // Changement de conversation : on repart de zéro.
  useEffect(() => {
    activeConversation.current = conversationId;
    setMessages([]);
    setLoading(true);
    setReplyTo(null);
    setEditing(null);
    setConfirmDelete(null);
    setTypingIds([]);
    setNewCount(0);
    setNearBottom(true);
    setPinIndex(0);
    prevLastIdRef.current = null;
    atBottomRef.current = true;
    for (const t of typingTimers.current.values()) clearTimeout(t);
    typingTimers.current.clear();
    void loadMessages().then(() => markRead());
  }, [conversationId, loadMessages, markRead]);

  useEffect(() => {
    const timers = typingTimers.current;
    return () => {
      for (const t of timers.values()) clearTimeout(t);
      if (toastTimer.current) clearTimeout(toastTimer.current);
      if (highlightTimer.current) clearTimeout(highlightTimer.current);
    };
  }, []);

  useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === "visible") void markRead();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [markRead]);

  const scrollToBottom = useCallback((smooth: boolean) => {
    const el = scrollRef.current;
    if (!el) return;
    el.scrollTo({
      top: el.scrollHeight,
      behavior: smooth && !prefersReducedMotion() ? "smooth" : "auto",
    });
  }, []);

  const lastMessage = messages[messages.length - 1];
  const lastId = lastMessage?.id ?? null;
  const lastSenderId = lastMessage?.senderId ?? null;

  // Collé en bas seulement si on y était déjà (ou si je viens d’écrire).
  useEffect(() => {
    if (loading) return;
    const prev = prevLastIdRef.current;
    prevLastIdRef.current = lastId;
    if (lastId === prev) return;
    if (prev === null) {
      scrollToBottom(false);
      return;
    }
    if (atBottomRef.current || lastSenderId === currentUserId) {
      scrollToBottom(true);
    } else {
      setNewCount((c) => c + 1);
    }
  }, [lastId, lastSenderId, loading, currentUserId, scrollToBottom]);

  useEffect(() => {
    if (typingIds.length > 0 && atBottomRef.current) scrollToBottom(true);
  }, [typingIds.length, scrollToBottom]);

  const onScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    const dist = el.scrollHeight - el.scrollTop - el.clientHeight;
    const bottom = dist < BOTTOM_THRESHOLD;
    atBottomRef.current = bottom;
    setNearBottom(dist < 200);
    if (bottom) setNewCount(0);
  }, []);

  const appendOrReplace = useCallback((msg: MessagingMessageDto) => {
    setMessages((prev) => {
      const idx = prev.findIndex((m) => m.id === msg.id);
      if (idx >= 0) {
        const copy = [...prev];
        copy[idx] = { ...prev[idx], ...msg };
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
        if (detail.message.senderId !== currentUserId) {
          void markRead();
        }
        // Le serveur peut compléter nom / mentions / accusés : on resynchronise.
        void loadMessages();
        const sender = detail.message.senderId;
        const t = typingTimers.current.get(sender);
        if (t) clearTimeout(t);
        typingTimers.current.delete(sender);
        setTypingIds((ids) => ids.filter((id) => id !== sender));
      }
      if (detail.type === "typing" && detail.userId && detail.userId !== currentUserId) {
        const uid = detail.userId;
        setTypingIds((ids) => (ids.includes(uid) ? ids : [...ids, uid]));
        const prev = typingTimers.current.get(uid);
        if (prev) clearTimeout(prev);
        typingTimers.current.set(
          uid,
          setTimeout(() => {
            typingTimers.current.delete(uid);
            setTypingIds((ids) => ids.filter((id) => id !== uid));
          }, 4000),
        );
      }
      if (
        (detail.type === "delivered" || detail.type === "read") &&
        detail.userId &&
        detail.userId !== currentUserId
      ) {
        void loadMessages();
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
    }).catch(() => undefined);
  }, [conversationId]);

  const onReact = useCallback(
    async (message: MessagingMessageDto, emoji: string) => {
      await fetch(`/api/messaging/messages/${message.id}/reactions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ emoji }),
      }).catch(() => undefined);
      void loadMessages();
    },
    [loadMessages],
  );

  const onPin = useCallback(
    async (message: MessagingMessageDto) => {
      const pinned = !message.pinnedAt;
      try {
        const res = await fetch(`/api/messaging/messages/${message.id}/pin`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ pinned }),
        });
        if (!res.ok) throw new Error("pin");
        const data = (await res.json()) as { message?: MessagingMessageDto };
        if (data.message) appendOrReplace(data.message);
        else void loadMessages();
        showToast(pinned ? "Message épinglé" : "Message désépinglé");
      } catch {
        showToast("Épinglage impossible");
      }
    },
    [appendOrReplace, loadMessages, showToast],
  );

  const onCopy = useCallback(
    async (message: MessagingMessageDto) => {
      try {
        await navigator.clipboard.writeText(message.body ?? "");
        showToast("Texte copié");
      } catch {
        showToast("Copie impossible");
      }
    },
    [showToast],
  );

  const confirmAndDelete = async () => {
    const target = confirmDelete;
    setConfirmDelete(null);
    if (!target) return;
    const res = await fetch(`/api/messaging/messages/${target.id}`, { method: "DELETE" }).catch(
      () => null,
    );
    if (!res || !res.ok) showToast("Suppression impossible");
    void loadMessages();
  };

  const onJumpTo = useCallback((id: string) => {
    const el = document.getElementById(`msg-${id}`);
    if (!el) return;
    el.scrollIntoView({ block: "center", behavior: prefersReducedMotion() ? "auto" : "smooth" });
    setHighlightId(id);
    if (highlightTimer.current) clearTimeout(highlightTimer.current);
    highlightTimer.current = setTimeout(() => setHighlightId(null), 1800);
  }, []);

  const onForward = useCallback((m: MessagingMessageDto) => forwardRef.current?.(m), []);
  const onReply = useCallback((m: MessagingMessageDto) => setReplyTo(m), []);
  const onEdit = useCallback((m: MessagingMessageDto) => {
    setReplyTo(null);
    setEditing(m);
  }, []);
  const onDeleteRequest = useCallback((m: MessagingMessageDto) => setConfirmDelete(m), []);

  const requestEditLast = useCallback(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      const m = messages[i];
      if (m.senderId === currentUserId && m.type === "text" && !m.deletedAt) {
        onEdit(m);
        return;
      }
    }
  }, [messages, currentUserId, onEdit]);

  const items = useMemo(() => buildItems(messages), [messages]);

  const lastMineId = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      const m = messages[i];
      if (m.senderId === currentUserId && m.type !== "system" && !m.deletedAt) return m.id;
    }
    return null;
  }, [messages, currentUserId]);

  const pinned = useMemo(() => {
    const map = new Map<string, MessagingMessageDto>();
    for (const m of pinnedMessages ?? []) if (m.pinnedAt && !m.deletedAt) map.set(m.id, m);
    for (const m of messages) {
      if (m.pinnedAt && !m.deletedAt) map.set(m.id, m);
      else map.delete(m.id);
    }
    return [...map.values()].sort(
      (a, b) => new Date(b.pinnedAt ?? 0).getTime() - new Date(a.pinnedAt ?? 0).getTime(),
    );
  }, [messages, pinnedMessages]);

  const currentPin = pinned.length > 0 ? pinned[pinIndex % pinned.length] : null;

  const shell =
    variant === "mobile-full"
      ? "fixed inset-0 z-[140] flex flex-col bg-[var(--dash-surface)]"
      : variant === "dock"
        ? "flex h-[420px] w-[340px] flex-col overflow-hidden rounded-t-[1.75rem] border border-black/6 bg-[var(--dash-surface)] shadow-[0_16px_40px_rgba(15,23,42,0.28)]"
        : "flex h-full min-h-0 flex-col overflow-hidden rounded-[1.75rem] border border-black/6 bg-white/80 shadow-[0_1px_0_rgba(0,0,0,0.03)] backdrop-blur-xl";

  const subtitle =
    kind === "group"
      ? `${memberCount && memberCount > 0 ? memberCount : Math.max(membersPreview.length + 1, 2)} membres`
      : presenceStatus === "online"
        ? "En ligne"
        : presenceStatus === "away"
          ? "Absent"
          : presenceStatus === "busy"
            ? "Occupé"
            : presenceStatus === "dnd"
              ? "Ne pas déranger"
              : "Hors ligne";

  const typingLabel = useMemo(() => {
    const names = typingIds.map((id) => membersById.get(id)?.name);
    if (kind !== "group") return `${peer?.name ?? names[0] ?? "Contact"} écrit…`;
    const known = names.filter((n): n is string => Boolean(n)).map(firstName);
    if (known.length === 0) return "Quelqu’un écrit…";
    if (typingIds.length === 1) return `${known[0]} écrit…`;
    if (typingIds.length === 2 && known.length === 2) return `${known[0]} et ${known[1]} écrivent…`;
    return "Plusieurs personnes écrivent…";
  }, [typingIds, membersById, kind, peer]);

  const iconBtn =
    "rounded-2xl p-2 outline-none transition hover:bg-black/[0.05] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50";

  return (
    <div {...{ [MESSAGING_ROOT_ATTR]: "" }} className={`${variant === "mobile-full" ? "" : "relative"} ${shell} ${className}`}>
      <header className="flex items-center gap-2.5 border-b border-black/5 bg-[#eceeea]/60 px-3.5 py-3">
        {kind === "group" ? (
          <div className="relative flex h-10 w-10 shrink-0 overflow-hidden rounded-2xl bg-[var(--dash-ink)]">
            {membersPreview.slice(0, 2).length === 0 ? (
              <span className="flex h-full w-full items-center justify-center text-xs font-bold text-white">
                <IconUsers className="h-4 w-4" />
              </span>
            ) : (
              <div className="absolute inset-0 grid grid-cols-2">
                {membersPreview.slice(0, 2).map((m) => (
                  <div key={m.id} className="overflow-hidden bg-neutral-300">
                    {m.imageUrl ? (
                      // eslint-disable-next-line @next/next/no-img-element
                      <img src={m.imageUrl} alt="" className="h-full w-full object-cover" />
                    ) : (
                      <span className="flex h-full w-full items-center justify-center text-[8px] font-bold text-[var(--dash-ink)]">
                        {m.name.slice(0, 1).toUpperCase()}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        ) : (
          <MessagingPresenceBadge status={presenceStatus} dotClassName="h-2.5 w-2.5">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-[var(--dash-ink)] text-sm font-semibold text-white">
              {peer?.imageUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={peer.imageUrl} alt="" className="h-full w-full object-cover" />
              ) : (
                (peer?.name || title || "?").slice(0, 1).toUpperCase()
              )}
            </div>
          </MessagingPresenceBadge>
        )}
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-semibold tracking-tight text-[var(--dash-ink)]">
            {title}
          </p>
          <p className="truncate text-[11px] font-medium text-[var(--dash-mid)]">
            {typingIds.length > 0 ? typingLabel : subtitle}
          </p>
        </div>
        {onStartVideoCall ? (
          <button
            type="button"
            title="Visio"
            aria-label="Démarrer une visio"
            className={`${iconBtn} text-[var(--dash-ink)]`}
            onClick={onStartVideoCall}
          >
            <IconVideoCall className="h-4 w-4" />
          </button>
        ) : null}
        {onOpenDetails ? (
          <button
            type="button"
            title="Informations"
            aria-label="Informations sur la conversation"
            className={`${iconBtn} text-[var(--dash-ink)]`}
            onClick={onOpenDetails}
          >
            <IconInfo className="h-4 w-4" />
          </button>
        ) : null}
        {onMinimize ? (
          <button
            type="button"
            aria-label="Réduire"
            className={`${iconBtn} text-neutral-500`}
            onClick={onMinimize}
          >
            <IconMinus className="h-4 w-4" />
          </button>
        ) : null}
        {onClose ? (
          <button
            type="button"
            aria-label="Fermer"
            className={`${iconBtn} text-neutral-500`}
            onClick={onClose}
          >
            <IconX className="h-4 w-4" />
          </button>
        ) : null}
      </header>

      {currentPin ? (
        <button
          type="button"
          onClick={() => {
            onJumpTo(currentPin.id);
            if (pinned.length > 1) setPinIndex((i) => (i + 1) % pinned.length);
          }}
          aria-label="Aller au message épinglé"
          className="flex w-full items-center gap-2 border-b border-black/5 bg-[var(--dash-lime)]/20 px-3.5 py-1.5 text-left text-xs outline-none transition hover:bg-[var(--dash-lime)]/35 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--dash-ink)]/50"
        >
          <IconPin className="h-3.5 w-3.5 shrink-0 text-[var(--dash-ink)]" />
          <span className="shrink-0 font-semibold text-[var(--dash-ink)]">
            Message épinglé{pinned.length > 1 ? ` (${(pinIndex % pinned.length) + 1}/${pinned.length})` : ""}
          </span>
          <span className="min-w-0 flex-1 truncate text-[var(--dash-mid)]">
            {currentPin.senderName ? `${firstName(currentPin.senderName)} : ` : ""}
            {messagePreview(currentPin)}
          </span>
        </button>
      ) : null}

      <div className="relative min-h-0 flex-1">
        <div
          ref={scrollRef}
          onScroll={onScroll}
          role="log"
          aria-live="polite"
          aria-label="Messages"
          className="h-full overflow-y-auto px-3.5 pb-3 pt-6"
        >
          {loading ? (
            <p className="text-center text-xs text-neutral-500">Chargement…</p>
          ) : messages.length === 0 ? (
            <p className="text-center text-xs text-neutral-500">Aucun message — dites bonjour !</p>
          ) : (
            items.map((it) => {
              if (it.kind === "day") {
                return (
                  <div key={it.key} className="my-3 flex justify-center">
                    <span className="rounded-full bg-black/[0.06] px-3 py-0.5 text-[11px] font-medium capitalize text-[var(--dash-mid)]">
                      {it.label}
                    </span>
                  </div>
                );
              }
              if (it.kind === "system") {
                return (
                  <p
                    key={it.key}
                    className="my-2 px-6 text-center text-[11px] italic text-neutral-500"
                  >
                    {it.message.body}
                  </p>
                );
              }
              const m = it.message;
              return (
                <MessagingMessageBubble
                  key={it.key}
                  message={m}
                  isMine={m.senderId === currentUserId}
                  currentUserId={currentUserId}
                  kind={kind}
                  firstInGroup={it.firstInGroup}
                  isLastMine={m.id === lastMineId}
                  highlighted={highlightId === m.id}
                  membersById={membersById}
                  onReply={onReply}
                  onForward={onForward}
                  onEdit={onEdit}
                  onDelete={onDeleteRequest}
                  onReact={onReact}
                  onPin={onPin}
                  onCopy={onCopy}
                  onJumpTo={onJumpTo}
                />
              );
            })
          )}
          {typingIds.length > 0 ? (
            <div className="mt-3">
              <TypingBubble label={typingLabel} />
            </div>
          ) : null}
        </div>

        {!nearBottom || newCount > 0 ? (
          <button
            type="button"
            onClick={() => {
              scrollToBottom(true);
              setNewCount(0);
            }}
            aria-label={newCount > 0 ? "Aller aux nouveaux messages" : "Aller au dernier message"}
            className="absolute bottom-3 left-1/2 z-10 flex -translate-x-1/2 items-center gap-1.5 rounded-full bg-[var(--dash-ink)] px-3 py-1.5 text-xs font-semibold text-white shadow-[0_8px_24px_-8px_rgba(0,0,0,0.5)] outline-none transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-[var(--dash-lime)]"
          >
            <IconArrowDown className="h-3.5 w-3.5" />
            {newCount > 0 ? "Nouveaux messages" : null}
          </button>
        ) : null}
      </div>

      <MessagingComposer
        conversationId={conversationId}
        replyTo={replyTo}
        onClearReply={() => setReplyTo(null)}
        editing={editing}
        onClearEdit={() => setEditing(null)}
        onSent={() => void loadMessages()}
        onTyping={sendTyping}
        members={mentionMembers}
        currentUserId={currentUserId}
        onRequestEditLast={requestEditLast}
      />

      {toast ? (
        <div
          role="status"
          className="pointer-events-none absolute inset-x-0 bottom-24 z-30 flex justify-center"
        >
          <span className="rounded-full bg-[var(--dash-ink)] px-3 py-1.5 text-xs font-medium text-white shadow-lg">
            {toast}
          </span>
        </div>
      ) : null}

      {confirmDelete ? (
        <div
          className="absolute inset-0 z-40 flex items-center justify-center bg-black/30 p-4"
          onKeyDown={(e) => {
            if (e.key === "Escape") setConfirmDelete(null);
          }}
        >
          <div
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="msg-delete-title"
            className="w-full max-w-xs rounded-2xl bg-white p-4 shadow-[0_20px_50px_-12px_rgba(0,0,0,0.5)]"
          >
            <p id="msg-delete-title" className="text-sm font-semibold text-[var(--dash-ink)]">
              Supprimer ce message pour tout le monde ?
            </p>
            <p className="mt-1 text-xs text-[var(--dash-mid)]">Cette action est définitive.</p>
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                autoFocus
                onClick={() => setConfirmDelete(null)}
                className="rounded-xl px-3 py-1.5 text-sm font-medium text-[var(--dash-ink)] outline-none hover:bg-black/[0.05] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50"
              >
                Annuler
              </button>
              <button
                type="button"
                onClick={() => void confirmAndDelete()}
                className="rounded-xl bg-rose-600 px-3 py-1.5 text-sm font-semibold text-white outline-none hover:bg-rose-700 focus-visible:ring-2 focus-visible:ring-rose-400"
              >
                Supprimer
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

export type { MessagingConversationDto };
