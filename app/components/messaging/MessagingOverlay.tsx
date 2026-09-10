"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import {
  MESSAGING_FAB_CLASS,
  MESSAGING_FAB_POSITION,
  MESSAGING_MAX_DOCKED,
  MESSAGING_MAX_DOCKED_NARROW,
  MESSAGING_NARROW_MQ,
  MESSAGING_RECENT_LIST,
  messagingDockStorageKey,
  messagingHeadsExpandedKey,
  pushDockedConversation,
  readDockedConversationIds,
  removeDockedConversation,
  writeDockedConversationIds,
  writeHeadsExpanded,
} from "@/app/lib/messaging/dock";
import type {
  MessagingConversationDto,
  MessagingMessageDto,
  MessagingPeer,
  MessagingSseEvent,
} from "@/app/lib/messaging/types";
import { useMessagingConversations, useMessagingStream } from "./useMessagingData";
import MessagingMainPanel from "./MessagingMainPanel";
import MessagingConversationPanel from "./MessagingConversationPanel";
import { IconMessageCircle } from "./MessagingIcons";

type OpenPanel = {
  conversation: MessagingConversationDto;
};

function ConversationAvatar({
  conv,
  sizeClass = "h-14 w-14",
}: {
  conv: MessagingConversationDto;
  sizeClass?: string;
}) {
  if (conv.kind === "group") {
    const previews = conv.membersPreview.slice(0, 2);
    return (
      <div className={`relative ${sizeClass} overflow-hidden rounded-full bg-gradient-to-br from-sky-500 to-indigo-600`}>
        {previews.length === 0 ? (
          <span className="flex h-full w-full items-center justify-center text-sm font-bold text-white">
            {(conv.title || "G").slice(0, 1).toUpperCase()}
          </span>
        ) : (
          <div className="absolute inset-0 grid grid-cols-2">
            {previews.map((m, i) => (
              <div
                key={m.id}
                className={`overflow-hidden bg-slate-200 ${previews.length === 1 ? "col-span-2" : ""}`}
              >
                {m.imageUrl ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img src={m.imageUrl} alt="" className="h-full w-full object-cover" />
                ) : (
                  <span className="flex h-full w-full items-center justify-center text-[10px] font-bold text-slate-700">
                    {m.name.slice(0, 1).toUpperCase()}
                  </span>
                )}
                {i === 0 && previews.length === 1 ? null : null}
              </div>
            ))}
          </div>
        )}
      </div>
    );
  }

  const peer = conv.peer;
  return (
    <div className={`overflow-hidden rounded-full bg-slate-200 ${sizeClass}`}>
      {peer?.imageUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={peer.imageUrl} alt="" className="h-full w-full object-cover" />
      ) : (
        <span className="flex h-full w-full items-center justify-center text-sm font-semibold text-slate-700">
          {(peer?.name || conv.title || "?").slice(0, 1).toUpperCase()}
        </span>
      )}
    </div>
  );
}

export default function MessagingOverlay() {
  const { user, isLoaded, isSignedIn } = useSessionUser();
  const enabled = Boolean(isLoaded && isSignedIn && user);
  const currentUserId = user?.id ?? "";

  const storageKey = useMemo(() => messagingDockStorageKey(currentUserId), [currentUserId]);
  const headsKey = useMemo(() => messagingHeadsExpandedKey(currentUserId), [currentUserId]);

  const [panelOpen, setPanelOpen] = useState(false);
  const [headsExpanded, setHeadsExpanded] = useState(false);
  const [dockedIds, setDockedIds] = useState<string[]>([]);
  const [openPanels, setOpenPanels] = useState<OpenPanel[]>([]);
  const [mobileFull, setMobileFull] = useState<OpenPanel | null>(null);
  const [forwardMessage, setForwardMessage] = useState<MessagingMessageDto | null>(null);
  const [maxDock, setMaxDock] = useState(MESSAGING_MAX_DOCKED);

  const { conversations, refresh, totalUnread } = useMessagingConversations(enabled);

  useEffect(() => {
    if (!enabled) return;
    setDockedIds(readDockedConversationIds(storageKey));
    // Chat heads repliées par défaut (style Facebook) — le clic FAB les révèle.
    setHeadsExpanded(false);
    writeHeadsExpanded(headsKey, false);
  }, [enabled, storageKey, headsKey]);

  useEffect(() => {
    const mq = window.matchMedia(MESSAGING_NARROW_MQ);
    const apply = () =>
      setMaxDock(mq.matches ? MESSAGING_MAX_DOCKED_NARROW : MESSAGING_MAX_DOCKED);
    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, []);

  const persistDock = useCallback(
    (ids: string[]) => {
      setDockedIds(ids);
      writeDockedConversationIds(storageKey, ids);
    },
    [storageKey],
  );

  const setExpanded = useCallback(
    (expanded: boolean) => {
      setHeadsExpanded(expanded);
      writeHeadsExpanded(headsKey, expanded);
    },
    [headsKey],
  );

  const openConversation = useCallback(
    (conversation: MessagingConversationDto) => {
      const narrow = window.matchMedia(MESSAGING_NARROW_MQ).matches;
      if (narrow) {
        setMobileFull({ conversation });
        setPanelOpen(false);
        return;
      }
      persistDock(pushDockedConversation(dockedIds, conversation.id, maxDock));
      setExpanded(true);
      setOpenPanels((prev) => {
        const without = prev.filter((p) => p.conversation.id !== conversation.id);
        return [{ conversation }, ...without].slice(0, maxDock);
      });
      setPanelOpen(false);
    },
    [dockedIds, maxDock, persistDock, setExpanded],
  );

  const closePanel = useCallback(
    (conversationId: string) => {
      setOpenPanels((prev) => prev.filter((p) => p.conversation.id !== conversationId));
      persistDock(removeDockedConversation(dockedIds, conversationId));
    },
    [dockedIds, persistDock],
  );

  const startWithPeer = useCallback(
    async (peer: MessagingPeer) => {
      const res = await fetch("/api/messaging/conversations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ peerUserId: peer.id }),
      });
      if (!res.ok) return;
      const data = (await res.json()) as { conversation?: MessagingConversationDto };
      if (!data.conversation) return;
      await refresh();
      openConversation(data.conversation);
    },
    [openConversation, refresh],
  );

  const createGroup = useCallback(
    async (title: string, memberIds: string[]) => {
      const res = await fetch("/api/messaging/conversations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ kind: "group", title, memberIds }),
      });
      if (!res.ok) return;
      const data = (await res.json()) as { conversation?: MessagingConversationDto };
      if (!data.conversation) return;
      await refresh();
      openConversation(data.conversation);
    },
    [openConversation, refresh],
  );

  const handleSse = useCallback(
    (event: MessagingSseEvent) => {
      window.dispatchEvent(
        new CustomEvent("scolia-messaging-event", {
          detail: {
            type: event.type,
            conversationId: event.conversationId,
            ...(typeof event.payload === "object" && event.payload
              ? (event.payload as Record<string, unknown>)
              : {}),
          },
        }),
      );
      if (
        event.type === "message" ||
        event.type === "conversation_updated" ||
        event.type === "read" ||
        event.type === "reaction"
      ) {
        void refresh();
      }
    },
    [refresh],
  );

  useMessagingStream({
    enabled,
    onEvent: handleSse,
    onFallbackPoll: () => void refresh(),
  });

  /** Sync open panels with refreshed conversation data. */
  useEffect(() => {
    if (!conversations.length) return;
    setOpenPanels((prev) =>
      prev
        .map((p) => {
          const updated = conversations.find((c) => c.id === p.conversation.id);
          return updated ? { conversation: updated } : p;
        })
        .filter(Boolean),
    );
  }, [conversations]);

  const onMainFabClick = () => {
    if (panelOpen) {
      setPanelOpen(false);
      return;
    }
    if (headsExpanded) {
      setExpanded(false);
      return;
    }
    setExpanded(true);
    setPanelOpen(true);
  };

  const forwardToConversation = async (target: MessagingConversationDto) => {
    if (!forwardMessage) return;
    await fetch(`/api/messaging/conversations/${target.id}/messages`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        body: forwardMessage.body,
        type: forwardMessage.type,
        forwardedFromId: forwardMessage.id,
        attachments: forwardMessage.attachments.map((a) => ({
          s3Key: a.s3Key,
          mime: a.mime,
          size: a.size,
          fileName: a.fileName,
          width: a.width,
          height: a.height,
        })),
      }),
    });
    setForwardMessage(null);
    await refresh();
    openConversation(target);
  };

  if (!enabled || !currentUserId) return null;

  const recentForHeads = (() => {
    const fromDock = dockedIds
      .map((id) => conversations.find((c) => c.id === id))
      .filter((c): c is MessagingConversationDto => Boolean(c));
    if (fromDock.length >= maxDock) return fromDock.slice(0, maxDock);
    const extras = conversations.filter((c) => !fromDock.some((d) => d.id === c.id));
    return [...fromDock, ...extras].slice(0, maxDock);
  })();

  const recentTen = conversations.slice(0, MESSAGING_RECENT_LIST);

  return (
    <>
      {/* Fenêtres de chat dockées — style Facebook, au-dessus de la pile */}
      <div className="pointer-events-none fixed bottom-[12.5rem] right-4 z-[128] hidden flex-row-reverse items-end gap-3 sm:flex">
        {openPanels.map((p) => (
          <div key={p.conversation.id} className="pointer-events-auto animate-[fadeIn_0.2s_ease-out]">
            <MessagingConversationPanel
              conversationId={p.conversation.id}
              title={p.conversation.title}
              peer={p.conversation.peer}
              kind={p.conversation.kind}
              membersPreview={p.conversation.membersPreview}
              memberCount={p.conversation.memberCount}
              currentUserId={currentUserId}
              variant="dock"
              onClose={() => closePanel(p.conversation.id)}
              onMinimize={() =>
                setOpenPanels((prev) =>
                  prev.filter((x) => x.conversation.id !== p.conversation.id),
                )
              }
              onForwardRequest={setForwardMessage}
            />
          </div>
        ))}
      </div>

      {/* Colonne : chat heads (si expand) + FAB principal — alignée sur l’IA (right-4) */}
      <div className={`${MESSAGING_FAB_POSITION} flex flex-col-reverse items-center gap-3`}>
        <div className="relative">
          <button
            type="button"
            title="Messagerie"
            aria-label="Messagerie"
            aria-expanded={headsExpanded || panelOpen}
            className={`${MESSAGING_FAB_CLASS} border border-sky-300/40 bg-gradient-to-br from-sky-500 to-blue-700 text-white backdrop-blur-xl hover:border-sky-200/50`}
            onClick={onMainFabClick}
          >
            <span className="pointer-events-none absolute inset-[1px] rounded-full border border-white/20" />
            <IconMessageCircle className="relative h-7 w-7" />
          </button>
          {totalUnread > 0 ? (
            <span className="absolute -right-1 -top-1 flex h-5 min-w-5 items-center justify-center rounded-full bg-red-500 px-1 text-[10px] font-bold text-white ring-2 ring-white">
              {totalUnread > 99 ? "99+" : totalUnread}
            </span>
          ) : null}

          <MessagingMainPanel
            open={panelOpen}
            onClose={() => setPanelOpen(false)}
            conversations={recentTen}
            allConversations={conversations}
            onOpenConversation={openConversation}
            onStartWithPeer={(peer) => void startWithPeer(peer)}
            onCreateGroup={(title, ids) => void createGroup(title, ids)}
          />
        </div>

        {/* 3 dernières bulles — visibles uniquement quand expand */}
        {headsExpanded
          ? recentForHeads.map((conv, index) => {
              const panelIsOpen = openPanels.some((p) => p.conversation.id === conv.id);
              return (
                <button
                  key={conv.id}
                  type="button"
                  title={conv.title}
                  style={{ transitionDelay: `${index * 40}ms` }}
                  className={`${MESSAGING_FAB_CLASS} relative origin-bottom animate-[fadeInUp_0.25s_ease-out] ring-2 ${
                    panelIsOpen ? "ring-sky-400" : "ring-white/90"
                  }`}
                  onClick={() => {
                    if (panelIsOpen) {
                      setOpenPanels((prev) =>
                        prev.filter((p) => p.conversation.id !== conv.id),
                      );
                    } else {
                      openConversation(conv);
                    }
                  }}
                >
                  <ConversationAvatar conv={conv} />
                  {conv.unreadCount > 0 ? (
                    <span className="absolute -right-0.5 -top-0.5 flex h-5 min-w-5 items-center justify-center rounded-full bg-red-500 px-1 text-[10px] font-bold text-white ring-2 ring-white">
                      {conv.unreadCount > 9 ? "9+" : conv.unreadCount}
                    </span>
                  ) : null}
                </button>
              );
            })
          : null}
      </div>

      {mobileFull ? (
        <MessagingConversationPanel
          conversationId={mobileFull.conversation.id}
          title={mobileFull.conversation.title}
          peer={mobileFull.conversation.peer}
          kind={mobileFull.conversation.kind}
          membersPreview={mobileFull.conversation.membersPreview}
          memberCount={mobileFull.conversation.memberCount}
          currentUserId={currentUserId}
          variant="mobile-full"
          onClose={() => setMobileFull(null)}
          onForwardRequest={setForwardMessage}
        />
      ) : null}

      {forwardMessage ? (
        <div className="fixed inset-0 z-[150] flex items-center justify-center bg-black/40 p-4">
          <div className="max-h-[80vh] w-full max-w-md overflow-hidden rounded-2xl bg-white shadow-2xl">
            <div className="flex items-center justify-between border-b px-4 py-3">
              <h3 className="text-sm font-semibold">Transférer vers…</h3>
              <button
                type="button"
                className="text-xs text-slate-500"
                onClick={() => setForwardMessage(null)}
              >
                Annuler
              </button>
            </div>
            <div className="max-h-[60vh] overflow-y-auto p-2">
              {conversations.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left hover:bg-slate-50"
                  onClick={() => void forwardToConversation(c)}
                >
                  <ConversationAvatar conv={c} sizeClass="h-10 w-10" />
                  <span className="truncate text-sm font-medium">{c.title}</span>
                </button>
              ))}
            </div>
          </div>
        </div>
      ) : null}

      <style jsx global>{`
        @keyframes fadeInUp {
          from {
            opacity: 0;
            transform: translateY(12px) scale(0.9);
          }
          to {
            opacity: 1;
            transform: translateY(0) scale(1);
          }
        }
        @keyframes fadeIn {
          from {
            opacity: 0;
            transform: translateY(8px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }
      `}</style>
    </>
  );
}
