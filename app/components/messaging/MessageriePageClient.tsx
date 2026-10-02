"use client";

import { useEffect, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import type {
  MessagingConversationDto,
  MessagingMessageDto,
  MessagingPeer,
} from "@/app/lib/messaging/types";
import {
  useMessagingConversations,
  useMessagingStream,
} from "@/app/components/messaging/useMessagingData";
import MessagingConversationPanel from "@/app/components/messaging/MessagingConversationPanel";
import {
  MessagingCallProvider,
  useMessagingCall,
} from "@/app/components/messaging/MessagingCallProvider";
import MessagingCallOverlay from "@/app/components/messaging/MessagingCallOverlay";
import MessagingCallStage from "@/app/components/messaging/MessagingCallStage";
import MessagingPresenceBadge from "@/app/components/messaging/MessagingPresenceBadge";
import MessagingStatusPicker from "@/app/components/messaging/MessagingStatusPicker";
import { useMessagingPresence } from "@/app/components/messaging/useMessagingPresence";
import ChannelsPanel from "@/app/components/messaging/ChannelsPanel";
import {
  IconSearch,
  IconMessageCircle,
  IconUsers,
  IconHash,
  IconPlus,
} from "@/app/components/messaging/MessagingIcons";

type Section = "messages" | "salons";

function parseSection(raw: string | null): Section {
  return raw === "salons" ? "salons" : "messages";
}

export default function MessageriePageClient() {
  const { user, isLoaded, isSignedIn } = useSessionUser();
  const enabled = Boolean(isLoaded && isSignedIn && user);
  const currentUserId = user?.id ?? "";

  if (!isLoaded) {
    return (
      <DashboardThemeRoot>
        <p className="p-8 text-sm text-neutral-500">Chargement…</p>
      </DashboardThemeRoot>
    );
  }

  if (!isSignedIn || !user) {
    return (
      <DashboardThemeRoot>
        <p className="p-8 text-sm text-neutral-500">
          Connectez-vous pour accéder à la messagerie.
        </p>
      </DashboardThemeRoot>
    );
  }

  return (
    <MessagingCallProvider currentUserId={currentUserId}>
      <MessageriePageBody currentUserId={currentUserId} enabled={enabled} />
    </MessagingCallProvider>
  );
}

function MessageriePageBody({
  currentUserId,
  enabled,
}: {
  currentUserId: string;
  enabled: boolean;
}) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const section = parseSection(searchParams.get("section"));

  const { handleSseEvent, startCall, call } = useMessagingCall();
  const callBusy = Boolean(call && call.phase !== "idle");
  const { conversations, refresh, totalUnread } = useMessagingConversations(enabled);
  const {
    statusForConversation,
    statusOf,
    applySseEvent: applyPresenceSse,
    myPresence,
    setManualStatus,
  } = useMessagingPresence({
    enabled,
    currentUserId,
    conversations,
    callBusy,
  });
  const [selected, setSelected] = useState<MessagingConversationDto | null>(null);
  const [users, setUsers] = useState<MessagingPeer[]>([]);
  const [query, setQuery] = useState("");
  const [forwardMessage, setForwardMessage] = useState<MessagingMessageDto | null>(null);
  const [showNewGroup, setShowNewGroup] = useState(false);
  const [groupTitle, setGroupTitle] = useState("");
  const [groupMembers, setGroupMembers] = useState<string[]>([]);
  const [creatingGroup, setCreatingGroup] = useState(false);

  const setSection = (next: Section) => {
    const params = new URLSearchParams(searchParams.toString());
    if (next === "messages") params.delete("section");
    else params.set("section", "salons");
    const qs = params.toString();
    router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
  };

  useEffect(() => {
    if (!enabled) return;
    fetch("/api/messaging/users", { cache: "no-store" })
      .then((r) => r.json())
      .then((data: { users?: MessagingPeer[] }) => {
        setUsers(Array.isArray(data.users) ? data.users : []);
      })
      .catch(() => setUsers([]));
  }, [enabled]);

  useEffect(() => {
    if (!selected && conversations.length > 0) {
      setSelected(conversations[0] ?? null);
    } else if (selected) {
      const updated = conversations.find((c) => c.id === selected.id);
      if (updated) setSelected(updated);
    }
  }, [conversations, selected]);

  useMessagingStream({
    enabled,
    onEvent: (ev) => {
      if (ev.type.startsWith("call_")) {
        handleSseEvent(ev);
      }
      applyPresenceSse(ev);

      if (ev.type === "message" && ev.conversationId) {
        const payload =
          typeof ev.payload === "object" && ev.payload
            ? (ev.payload as {
                message?: { id?: string; senderId?: string };
              })
            : null;
        const msg = payload?.message;
        if (msg?.senderId && msg.senderId !== currentUserId) {
          void fetch(`/api/messaging/conversations/${ev.conversationId}/delivered`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ messageId: msg.id ?? null }),
          }).catch(() => undefined);
        }
      }

      window.dispatchEvent(
        new CustomEvent("scolia-messaging-event", {
          detail: {
            type: ev.type,
            conversationId: ev.conversationId,
            ...(typeof ev.payload === "object" && ev.payload
              ? (ev.payload as Record<string, unknown>)
              : {}),
          },
        }),
      );
      if (
        ev.type === "message" ||
        ev.type === "conversation_updated" ||
        ev.type === "read" ||
        ev.type === "delivered" ||
        ev.type === "reaction"
      ) {
        void refresh();
      }
    },
    onFallbackPoll: () => void refresh(),
  });

  const startWithPeer = async (peer: MessagingPeer) => {
    const res = await fetch("/api/messaging/conversations", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ peerUserId: peer.id }),
    });
    if (!res.ok) return;
    const data = (await res.json()) as { conversation?: MessagingConversationDto };
    await refresh();
    if (data.conversation) setSelected(data.conversation);
  };

  const createGroup = async () => {
    if (creatingGroup || groupTitle.trim().length < 2 || groupMembers.length < 1) return;
    setCreatingGroup(true);
    try {
      const res = await fetch("/api/messaging/conversations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind: "group",
          title: groupTitle.trim(),
          memberIds: groupMembers,
        }),
      });
      if (!res.ok) return;
      const data = (await res.json()) as { conversation?: MessagingConversationDto };
      await refresh();
      if (data.conversation) setSelected(data.conversation);
      setShowNewGroup(false);
      setGroupTitle("");
      setGroupMembers([]);
    } finally {
      setCreatingGroup(false);
    }
  };

  useEffect(() => {
    if (!call?.conversationId) return;
    const conv = conversations.find((c) => c.id === call.conversationId);
    if (conv) setSelected(conv);
  }, [call?.conversationId, conversations]);

  const q = query.trim().toLowerCase();
  const filteredUsers = q
    ? users.filter(
        (u) => u.name.toLowerCase().includes(q) || u.email.toLowerCase().includes(q),
      )
    : [];

  const inSplitCall = Boolean(call && call.phase !== "idle" && call.viewMode === "split");

  return (
    <DashboardThemeRoot>
      <MessagingCallOverlay embedSplitInPage />
      <div className="mx-auto flex h-[calc(100dvh-5.5rem)] w-full max-w-[90rem] flex-col gap-3 p-3 sm:p-4 lg:p-5">
        <header className="flex flex-wrap items-center justify-between gap-3 px-1">
          <div className="flex min-w-0 items-center gap-3">
            <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-[var(--dash-ink)] text-white">
              <IconMessageCircle className="h-5 w-5" />
            </span>
            <div className="min-w-0">
              <h1 className="truncate text-lg font-semibold tracking-tight text-[var(--dash-ink)] sm:text-xl">
                Messagerie
              </h1>
              <p className="text-[11px] font-medium text-[var(--dash-mid)]">
                {section === "salons"
                  ? "Salons d’équipe"
                  : totalUnread > 0
                    ? `${totalUnread} non lu${totalUnread > 1 ? "s" : ""}`
                    : "Discussions privées et groupes"}
                {inSplitCall ? " · Visio en cours" : ""}
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <nav
              className="inline-flex rounded-2xl border border-black/6 bg-[#eceeea] p-1"
              aria-label="Sections messagerie"
            >
              <button
                type="button"
                onClick={() => setSection("messages")}
                className={`inline-flex items-center gap-1.5 rounded-xl px-3 py-1.5 text-xs font-semibold transition ${
                  section === "messages"
                    ? "bg-[var(--dash-ink)] text-white shadow-sm"
                    : "text-[var(--dash-ink)] hover:bg-white/70"
                }`}
              >
                <IconMessageCircle className="h-3.5 w-3.5" />
                Messages
              </button>
              <button
                type="button"
                onClick={() => setSection("salons")}
                className={`inline-flex items-center gap-1.5 rounded-xl px-3 py-1.5 text-xs font-semibold transition ${
                  section === "salons"
                    ? "bg-[var(--dash-ink)] text-white shadow-sm"
                    : "text-[var(--dash-ink)] hover:bg-white/70"
                }`}
              >
                <IconHash className="h-3.5 w-3.5" />
                Salons
              </button>
            </nav>

            {section === "messages" ? (
              <>
                <div className="w-[200px] sm:w-[220px]">
                  <MessagingStatusPicker
                    myPresence={myPresence}
                    onSetManual={setManualStatus}
                  />
                </div>
                <button
                  type="button"
                  className="inline-flex items-center gap-1.5 rounded-2xl bg-[var(--dash-ink)] px-3 py-2 text-xs font-semibold text-white transition hover:opacity-90"
                  onClick={() => setShowNewGroup(true)}
                >
                  <IconPlus className="h-3.5 w-3.5" />
                  Groupe
                </button>
              </>
            ) : null}
          </div>
        </header>

        {section === "salons" ? (
          <ChannelsPanel embedded className="min-h-0 flex-1" />
        ) : (
          <div
            className={`grid min-h-0 flex-1 gap-3 transition-opacity duration-200 ${
              inSplitCall
                ? "lg:grid-cols-[minmax(260px,17rem)_minmax(0,1.15fr)_minmax(280px,1fr)]"
                : "md:grid-cols-[minmax(260px,17rem)_minmax(0,1fr)]"
            }`}
          >
            <aside className="flex min-h-0 flex-col overflow-hidden rounded-[1.75rem] border border-black/6 bg-[#eceeea] shadow-[0_1px_0_rgba(0,0,0,0.03)]">
              <div className="border-b border-black/5 p-2.5">
                <div className="relative">
                  <IconSearch className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-neutral-400" />
                  <input
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    placeholder="Chercher un collègue…"
                    className="w-full rounded-2xl border border-black/6 bg-white/80 py-2 pl-9 pr-3 text-sm outline-none transition focus:border-[var(--dash-ink)]"
                  />
                </div>
              </div>
              <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2 pt-1">
                {filteredUsers.length > 0 ? (
                  <div className="mb-2">
                    <p className="px-2 py-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--dash-mid)]">
                      Personnel
                    </p>
                    {filteredUsers.slice(0, 15).map((u) => (
                      <button
                        key={u.id}
                        type="button"
                        className="flex w-full items-center gap-2.5 rounded-2xl px-2.5 py-2 text-left transition hover:bg-white/70"
                        onClick={() => void startWithPeer(u)}
                      >
                        <MessagingPresenceBadge
                          status={statusOf(u.id)}
                          dotClassName="h-2.5 w-2.5"
                        >
                          <span className="flex h-9 w-9 items-center justify-center overflow-hidden rounded-2xl bg-[var(--dash-ink)] text-xs font-semibold text-white">
                            {u.name.slice(0, 1).toUpperCase()}
                          </span>
                        </MessagingPresenceBadge>
                        <span className="truncate text-sm font-medium text-[var(--dash-ink)]">
                          {u.name}
                        </span>
                      </button>
                    ))}
                  </div>
                ) : null}
                <p className="px-2 py-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--dash-mid)]">
                  Conversations
                </p>
                {conversations.length === 0 ? (
                  <p className="px-3 py-6 text-xs leading-relaxed text-neutral-500">
                    Aucune conversation. Cherchez un collègue pour démarrer.
                  </p>
                ) : (
                  conversations.map((c) => {
                    const active = selected?.id === c.id;
                    return (
                      <button
                        key={c.id}
                        type="button"
                        className={`flex w-full items-center gap-2.5 rounded-2xl px-2.5 py-2.5 text-left transition ${
                          active
                            ? "bg-white shadow-[0_1px_0_rgba(0,0,0,0.04)]"
                            : "hover:bg-white/70"
                        }`}
                        onClick={() => setSelected(c)}
                      >
                        <span className="relative shrink-0">
                          <MessagingPresenceBadge
                            status={statusForConversation(c)}
                            dotClassName="h-2.5 w-2.5"
                          >
                            <span className="flex h-10 w-10 items-center justify-center overflow-hidden rounded-2xl bg-[var(--dash-ink)] text-xs font-semibold text-white">
                              {c.kind === "group" ? (
                                c.membersPreview[0]?.imageUrl ? (
                                  // eslint-disable-next-line @next/next/no-img-element
                                  <img
                                    src={c.membersPreview[0].imageUrl}
                                    alt=""
                                    className="h-full w-full object-cover"
                                  />
                                ) : (
                                  (c.title || "G").slice(0, 1).toUpperCase()
                                )
                              ) : c.peer?.imageUrl ? (
                                // eslint-disable-next-line @next/next/no-img-element
                                <img
                                  src={c.peer.imageUrl}
                                  alt=""
                                  className="h-full w-full object-cover"
                                />
                              ) : (
                                (c.peer?.name || c.title || "?").slice(0, 1).toUpperCase()
                              )}
                            </span>
                          </MessagingPresenceBadge>
                          {c.unreadCount > 0 ? (
                            <span className="absolute -right-0.5 -top-0.5 z-10 min-w-[1rem] rounded-full bg-[var(--dash-lime)] px-1 text-center text-[9px] font-bold text-[var(--dash-ink)]">
                              {c.unreadCount}
                            </span>
                          ) : null}
                        </span>
                        <div className="min-w-0 flex-1">
                          <p className="truncate text-sm font-semibold text-[var(--dash-ink)]">
                            {c.title}
                            {c.kind === "group" ? (
                              <span className="ml-1.5 text-[10px] font-medium text-[var(--dash-mid)]">
                                groupe
                              </span>
                            ) : null}
                          </p>
                          <p className="truncate text-[11px] text-neutral-500">
                            {c.lastMessage?.body || "—"}
                          </p>
                        </div>
                      </button>
                    );
                  })
                )}
              </div>
            </aside>

            {inSplitCall ? (
              <section className="hidden min-h-0 overflow-hidden rounded-[1.75rem] border border-black/6 lg:block">
                <MessagingCallStage className="h-full" />
              </section>
            ) : null}

            <section className="min-h-0">
              {selected ? (
                <MessagingConversationPanel
                  conversationId={selected.id}
                  title={selected.title}
                  peer={selected.peer}
                  kind={selected.kind}
                  membersPreview={selected.membersPreview}
                  memberCount={selected.memberCount}
                  currentUserId={currentUserId}
                  presenceStatus={statusForConversation(selected)}
                  variant="page"
                  className="h-full"
                  onForwardRequest={setForwardMessage}
                  onStartVideoCall={() =>
                    void startCall({
                      conversationId: selected.id,
                      title: selected.title,
                      kind: selected.kind,
                    })
                  }
                />
              ) : (
                <div className="flex h-full items-center justify-center rounded-[1.75rem] border border-black/6 bg-white/80 text-sm text-neutral-400 shadow-[0_1px_0_rgba(0,0,0,0.03)] backdrop-blur-xl">
                  Sélectionnez ou démarrez une conversation
                </div>
              )}
              {inSplitCall ? (
                <div className="mt-3 overflow-hidden rounded-[1.75rem] border border-black/6 lg:hidden">
                  <MessagingCallStage className="h-[280px]" />
                </div>
              ) : null}
            </section>
          </div>
        )}
      </div>

      {showNewGroup ? (
        <div className="fixed inset-0 z-[150] flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm">
          <div className="max-h-[85vh] w-full max-w-md overflow-hidden rounded-[1.75rem] border border-black/6 bg-white shadow-2xl">
            <div className="flex items-center justify-between border-b border-black/5 px-4 py-3">
              <h3 className="text-sm font-semibold text-[var(--dash-ink)]">Nouveau groupe</h3>
              <button
                type="button"
                className="text-xs font-medium text-[var(--dash-mid)]"
                onClick={() => setShowNewGroup(false)}
              >
                Annuler
              </button>
            </div>
            <div className="space-y-3 p-4">
              <input
                value={groupTitle}
                onChange={(e) => setGroupTitle(e.target.value)}
                placeholder="Nom du groupe"
                className="w-full rounded-2xl border border-black/8 bg-[#eceeea]/50 px-3 py-2.5 text-sm outline-none focus:border-[var(--dash-ink)]"
              />
              <div className="max-h-56 space-y-0.5 overflow-y-auto rounded-2xl border border-black/6 p-1">
                {users.map((u) => {
                  const checked = groupMembers.includes(u.id);
                  return (
                    <label
                      key={u.id}
                      className={`flex cursor-pointer items-center gap-2 rounded-xl px-2.5 py-2 ${
                        checked ? "bg-[#eceeea]" : "hover:bg-black/[0.03]"
                      }`}
                    >
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={() =>
                          setGroupMembers((prev) =>
                            checked
                              ? prev.filter((id) => id !== u.id)
                              : [...prev, u.id],
                          )
                        }
                      />
                      <span className="truncate text-sm">{u.name}</span>
                    </label>
                  );
                })}
              </div>
              <button
                type="button"
                disabled={
                  creatingGroup || groupTitle.trim().length < 2 || groupMembers.length < 1
                }
                className="inline-flex w-full items-center justify-center gap-1.5 rounded-2xl bg-[var(--dash-ink)] py-2.5 text-sm font-semibold text-white disabled:opacity-40"
                onClick={() => void createGroup()}
              >
                <IconUsers className="h-4 w-4" />
                {creatingGroup ? "Création…" : "Créer"}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {forwardMessage ? (
        <div className="fixed inset-0 z-[150] flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm">
          <div className="max-h-[80vh] w-full max-w-md overflow-hidden rounded-[1.75rem] border border-black/6 bg-white shadow-xl">
            <div className="flex items-center justify-between border-b border-black/5 px-4 py-3">
              <h3 className="text-sm font-semibold text-[var(--dash-ink)]">Transférer vers…</h3>
              <button
                type="button"
                className="text-xs font-medium text-[var(--dash-mid)]"
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
                  className="flex w-full rounded-2xl px-3 py-2.5 text-left text-sm transition hover:bg-[#eceeea]"
                  onClick={async () => {
                    if (!forwardMessage) return;
                    await fetch(`/api/messaging/conversations/${c.id}/messages`, {
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
                    setSelected(c);
                  }}
                >
                  {c.title}
                </button>
              ))}
            </div>
          </div>
        </div>
      ) : null}
    </DashboardThemeRoot>
  );
}
