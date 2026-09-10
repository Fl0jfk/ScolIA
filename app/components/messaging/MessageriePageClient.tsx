"use client";

import { useCallback, useEffect, useState } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import { dash } from "@/app/lib/dashboard-brand";
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
import { IconSearch, IconMessageCircle, IconUsers } from "@/app/components/messaging/MessagingIcons";

export default function MessageriePageClient() {
  const { user, isLoaded, isSignedIn } = useSessionUser();
  const enabled = Boolean(isLoaded && isSignedIn && user);
  const currentUserId = user?.id ?? "";

  const { conversations, refresh, totalUnread } = useMessagingConversations(enabled);
  const [selected, setSelected] = useState<MessagingConversationDto | null>(null);
  const [users, setUsers] = useState<MessagingPeer[]>([]);
  const [query, setQuery] = useState("");
  const [forwardMessage, setForwardMessage] = useState<MessagingMessageDto | null>(null);
  const [showNewGroup, setShowNewGroup] = useState(false);
  const [groupTitle, setGroupTitle] = useState("");
  const [groupMembers, setGroupMembers] = useState<string[]>([]);
  const [creatingGroup, setCreatingGroup] = useState(false);

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

  const handleSse = useCallback(() => {
    void refresh();
  }, [refresh]);

  useMessagingStream({
    enabled,
    onEvent: (ev) => {
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
      handleSse();
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

  const q = query.trim().toLowerCase();
  const filteredUsers = q
    ? users.filter(
        (u) => u.name.toLowerCase().includes(q) || u.email.toLowerCase().includes(q),
      )
    : [];

  if (!isLoaded) {
    return (
      <DashboardThemeRoot>
        <p className="p-8 text-sm text-slate-500">Chargement…</p>
      </DashboardThemeRoot>
    );
  }

  if (!isSignedIn || !user) {
    return (
      <DashboardThemeRoot>
        <p className="p-8 text-sm text-slate-500">Connectez-vous pour accéder à la messagerie.</p>
      </DashboardThemeRoot>
    );
  }

  return (
    <DashboardThemeRoot>
      <div className="mx-auto flex h-[calc(100vh-6rem)] max-w-6xl flex-col gap-4 p-4 md:p-6">
        <header className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <IconMessageCircle className="h-6 w-6 text-sky-600" />
            <div>
              <h1 className={`text-xl font-semibold ${dash.ink}`}>Messagerie</h1>
              <p className="text-xs text-slate-500">
                Discussions 1:1 et groupes — personnel
                {totalUnread > 0 ? ` · ${totalUnread} non lu${totalUnread > 1 ? "s" : ""}` : ""}
              </p>
            </div>
          </div>
          <button
            type="button"
            className="inline-flex items-center gap-1.5 rounded-full bg-sky-600 px-3 py-1.5 text-xs font-semibold text-white shadow-sm hover:bg-sky-700"
            onClick={() => setShowNewGroup(true)}
          >
            <IconUsers className="h-3.5 w-3.5" />
            Nouveau groupe
          </button>
        </header>

        <div className="grid min-h-0 flex-1 gap-3 md:grid-cols-[320px_1fr]">
          <aside className="flex min-h-0 flex-col overflow-hidden rounded-xl bg-white ring-1 ring-slate-200">
            <div className="border-b border-slate-100 p-2">
              <div className="relative">
                <IconSearch className="pointer-events-none absolute left-2 top-2.5 h-4 w-4 text-slate-400" />
                <input
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder="Chercher un collègue…"
                  className="w-full rounded-lg border border-slate-200 py-2 pl-8 pr-2 text-sm outline-none focus:border-sky-400"
                />
              </div>
            </div>
            <div className="flex-1 overflow-y-auto p-1">
              {filteredUsers.length > 0 ? (
                <div className="mb-2">
                  <p className="px-2 py-1 text-[10px] font-semibold uppercase text-slate-400">
                    Personnel
                  </p>
                  {filteredUsers.slice(0, 15).map((u) => (
                    <button
                      key={u.id}
                      type="button"
                      className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left hover:bg-slate-50"
                      onClick={() => void startWithPeer(u)}
                    >
                      <span className="flex h-8 w-8 items-center justify-center rounded-full bg-sky-100 text-xs font-semibold text-sky-800">
                        {u.name.slice(0, 1).toUpperCase()}
                      </span>
                      <span className="truncate text-sm">{u.name}</span>
                    </button>
                  ))}
                </div>
              ) : null}
              <p className="px-2 py-1 text-[10px] font-semibold uppercase text-slate-400">
                Conversations
              </p>
              {conversations.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  className={`flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left ${
                    selected?.id === c.id ? "bg-sky-50" : "hover:bg-slate-50"
                  }`}
                  onClick={() => setSelected(c)}
                >
                  <span className="relative flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-sky-100 text-xs font-semibold text-sky-800">
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
                      <img src={c.peer.imageUrl} alt="" className="h-full w-full object-cover" />
                    ) : (
                      (c.peer?.name || c.title || "?").slice(0, 1).toUpperCase()
                    )}
                    {c.unreadCount > 0 ? (
                      <span className="absolute -right-0.5 -top-0.5 rounded-full bg-red-500 px-1 text-[9px] text-white">
                        {c.unreadCount}
                      </span>
                    ) : null}
                  </span>
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-slate-800">
                      {c.title}
                      {c.kind === "group" ? (
                        <span className="ml-1 text-[10px] font-normal text-indigo-500">groupe</span>
                      ) : null}
                    </p>
                    <p className="truncate text-[11px] text-slate-400">
                      {c.lastMessage?.body || "—"}
                    </p>
                  </div>
                </button>
              ))}
            </div>
          </aside>

          <section className="min-h-0">
            {selected ? (
              <MessagingConversationPanel
                conversationId={selected.id}
                title={selected.title}
                peer={selected.peer}
                kind={selected.kind}
                membersPreview={selected.membersPreview}
                currentUserId={currentUserId}
                variant="page"
                className="h-full"
                onForwardRequest={setForwardMessage}
              />
            ) : (
              <div className="flex h-full items-center justify-center rounded-xl bg-white text-sm text-slate-400 ring-1 ring-slate-200">
                Sélectionnez ou démarrez une conversation
              </div>
            )}
          </section>
        </div>
      </div>

      {showNewGroup ? (
        <div className="fixed inset-0 z-[150] flex items-center justify-center bg-black/40 p-4">
          <div className="max-h-[85vh] w-full max-w-md overflow-hidden rounded-2xl bg-white shadow-2xl">
            <div className="flex items-center justify-between border-b px-4 py-3">
              <h3 className="text-sm font-semibold">Nouveau groupe</h3>
              <button
                type="button"
                className="text-xs text-slate-500"
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
                className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm outline-none focus:border-sky-400"
              />
              <div className="max-h-56 space-y-0.5 overflow-y-auto rounded-xl border border-slate-100 p-1">
                {users.map((u) => {
                  const checked = groupMembers.includes(u.id);
                  return (
                    <label
                      key={u.id}
                      className={`flex cursor-pointer items-center gap-2 rounded-lg px-2 py-2 ${
                        checked ? "bg-sky-50" : "hover:bg-slate-50"
                      }`}
                    >
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={() =>
                          setGroupMembers((prev) =>
                            checked ? prev.filter((id) => id !== u.id) : [...prev, u.id],
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
                disabled={creatingGroup || groupTitle.trim().length < 2 || groupMembers.length < 1}
                className="w-full rounded-full bg-sky-600 py-2.5 text-sm font-semibold text-white disabled:opacity-40"
                onClick={() => void createGroup()}
              >
                {creatingGroup ? "Création…" : "Créer"}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {forwardMessage ? (
        <div className="fixed inset-0 z-[150] flex items-center justify-center bg-black/40 p-4">
          <div className="max-h-[80vh] w-full max-w-md overflow-hidden rounded-xl bg-white shadow-xl">
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
                  className="flex w-full rounded-lg px-3 py-2 text-left text-sm hover:bg-slate-50"
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
