"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import type {
  MessagingConversationDto,
  MessagingMyPresenceDto,
  MessagingPeer,
  MessagingPresenceStatus,
} from "@/app/lib/messaging/types";
import {
  IconSearch,
  IconMessageCircle,
  IconExternalLink,
  IconUsers,
  IconX,
} from "./MessagingIcons";
import MessagingPresenceBadge from "./MessagingPresenceBadge";
import MessagingStatusPicker from "./MessagingStatusPicker";

type Props = {
  open: boolean;
  onClose: () => void;
  /** 10 dernières conversations (liste principale style Facebook). */
  conversations: MessagingConversationDto[];
  allConversations: MessagingConversationDto[];
  onOpenConversation: (conversation: MessagingConversationDto) => void;
  onStartWithPeer: (peer: MessagingPeer) => void;
  onCreateGroup: (title: string, memberIds: string[]) => void;
  presenceOf?: (userId: string | null | undefined) => MessagingPresenceStatus;
  presenceForConversation?: (conv: MessagingConversationDto) => MessagingPresenceStatus;
  myPresence?: MessagingMyPresenceDto;
  onSetManualStatus?: (
    status: Exclude<MessagingPresenceStatus, "offline">,
    durationHours: 0 | 1 | 4 | 24,
  ) => Promise<unknown>;
};

type DirectoryUser = MessagingPeer;
type Tab = "recent" | "people" | "group";

function ConvAvatar({
  conv,
  unread = 0,
  presence = "offline",
}: {
  conv: MessagingConversationDto;
  unread?: number;
  presence?: MessagingPresenceStatus;
}) {
  const peer = conv.peer;
  const avatar =
    conv.kind === "group" ? (
      <div className="relative flex h-11 w-11 overflow-hidden rounded-full bg-gradient-to-br from-sky-500 to-indigo-600">
        {conv.membersPreview.slice(0, 2).length === 0 ? (
          <span className="flex h-full w-full items-center justify-center text-xs font-bold text-white">
            {(conv.title || "G").slice(0, 1).toUpperCase()}
          </span>
        ) : (
          <div className="absolute inset-0 grid grid-cols-2">
            {conv.membersPreview.slice(0, 2).map((m) => (
              <div key={m.id} className="overflow-hidden bg-slate-200">
                {m.imageUrl ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img src={m.imageUrl} alt="" className="h-full w-full object-cover" />
                ) : (
                  <span className="flex h-full w-full items-center justify-center text-[9px] font-bold text-slate-700">
                    {m.name.slice(0, 1).toUpperCase()}
                  </span>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    ) : (
      <div className="flex h-11 w-11 items-center justify-center overflow-hidden rounded-full bg-sky-100 text-sm font-semibold text-sky-800">
        {peer?.imageUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img src={peer.imageUrl} alt="" className="h-full w-full object-cover" />
        ) : (
          (peer?.name || conv.title || "?").slice(0, 1).toUpperCase()
        )}
      </div>
    );

  return (
    <div className="relative shrink-0">
      <MessagingPresenceBadge status={presence} dotClassName="h-3 w-3">
        {avatar}
      </MessagingPresenceBadge>
      {unread > 0 ? (
        <span className="absolute -right-1 -top-1 z-10 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-500 px-0.5 text-[9px] font-bold text-white ring-2 ring-white">
          {unread > 9 ? "9+" : unread}
        </span>
      ) : null}
    </div>
  );
}

function PeerAvatar({
  peer,
  presence = "offline",
}: {
  peer: MessagingPeer;
  presence?: MessagingPresenceStatus;
}) {
  return (
    <MessagingPresenceBadge status={presence} dotClassName="h-2.5 w-2.5">
      <div className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-sky-100 text-xs font-semibold text-sky-800">
        {peer.imageUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img src={peer.imageUrl} alt="" className="h-full w-full object-cover" />
        ) : (
          (peer.name || "?").slice(0, 1).toUpperCase()
        )}
      </div>
    </MessagingPresenceBadge>
  );
}

export default function MessagingMainPanel({
  open,
  onClose,
  conversations,
  allConversations,
  onOpenConversation,
  onStartWithPeer,
  onCreateGroup,
  presenceOf,
  presenceForConversation,
  myPresence,
  onSetManualStatus,
}: Props) {
  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<Tab>("recent");
  const [users, setUsers] = useState<DirectoryUser[]>([]);
  const [loadingUsers, setLoadingUsers] = useState(false);
  const [groupTitle, setGroupTitle] = useState("");
  const [groupMembers, setGroupMembers] = useState<string[]>([]);
  const [creatingGroup, setCreatingGroup] = useState(false);
  const [searchResults, setSearchResults] = useState<
    Array<{ messageId: string; conversationId: string; body: string | null; createdAt: string }>
  >([]);
  const [directoryPresence, setDirectoryPresence] = useState<
    Record<string, MessagingPresenceStatus>
  >({});

  useEffect(() => {
    if (!open) return;
    setLoadingUsers(true);
    fetch("/api/messaging/users", { cache: "no-store" })
      .then((r) => r.json())
      .then((data: { users?: DirectoryUser[] }) => {
        setUsers(Array.isArray(data.users) ? data.users : []);
      })
      .catch(() => setUsers([]))
      .finally(() => setLoadingUsers(false));
  }, [open]);

  useEffect(() => {
    if (!open || users.length === 0) return;
    const ids = users.map((u) => u.id).slice(0, 200);
    let cancelled = false;
    const load = () => {
      void fetch(
        `/api/messaging/presence?userIds=${encodeURIComponent(ids.join(","))}`,
        { cache: "no-store" },
      )
        .then((r) => r.json())
        .then((data: { presence?: Record<string, MessagingPresenceStatus> }) => {
          if (!cancelled && data.presence) setDirectoryPresence(data.presence);
        })
        .catch(() => undefined);
    };
    load();
    const timer = setInterval(load, 30_000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [open, users]);

  const peerPresence = (userId: string): MessagingPresenceStatus =>
    presenceOf?.(userId) ?? directoryPresence[userId] ?? "offline";

  const convPresence = (c: MessagingConversationDto): MessagingPresenceStatus =>
    presenceForConversation?.(c) ??
    (c.kind === "dm" ? peerPresence(c.peer?.id ?? "") : "offline");

  useEffect(() => {
    if (!open) {
      setQuery("");
      setTab("recent");
      setGroupTitle("");
      setGroupMembers([]);
      return;
    }
  }, [open]);

  useEffect(() => {
    if (!open || query.trim().length < 2) {
      setSearchResults([]);
      return;
    }
    const t = setTimeout(() => {
      fetch(`/api/messaging/search?q=${encodeURIComponent(query.trim())}`, { cache: "no-store" })
        .then((r) => r.json())
        .then(
          (data: {
            results?: Array<{
              messageId: string;
              conversationId: string;
              body: string | null;
              createdAt: string;
            }>;
          }) => {
            setSearchResults(Array.isArray(data.results) ? data.results : []);
          },
        )
        .catch(() => setSearchResults([]));
    }, 300);
    return () => clearTimeout(t);
  }, [query, open]);

  const filteredUsers = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return users.slice(0, 16);
    return users
      .filter(
        (u) =>
          u.name.toLowerCase().includes(q) ||
          u.email.toLowerCase().includes(q),
      )
      .slice(0, 24);
  }, [users, query]);

  const submitGroup = async () => {
    if (creatingGroup) return;
    setCreatingGroup(true);
    try {
      await onCreateGroup(groupTitle.trim(), groupMembers);
      setGroupTitle("");
      setGroupMembers([]);
      setTab("recent");
    } finally {
      setCreatingGroup(false);
    }
  };

  if (!open) return null;

  return (
    <div className="absolute bottom-0 right-[4.75rem] z-[131] flex h-[520px] w-[380px] origin-bottom-right animate-[fadeIn_0.18s_ease-out] flex-col overflow-hidden rounded-2xl bg-white/95 shadow-[0_20px_50px_rgba(15,23,42,0.28)] ring-1 ring-slate-200/80 backdrop-blur-xl max-sm:right-0 max-sm:bottom-[4.75rem] max-sm:w-[min(380px,calc(100vw-1.5rem))]">
      {/* Header style Messenger classic */}
      <header className="relative overflow-hidden bg-gradient-to-br from-sky-600 via-blue-600 to-indigo-700 px-4 pb-3 pt-3 text-white">
        <div className="pointer-events-none absolute -right-6 -top-8 h-28 w-28 rounded-full bg-white/10" />
        <div className="pointer-events-none absolute -bottom-10 left-8 h-24 w-24 rounded-full bg-sky-300/20" />
        <div className="relative flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <span className="flex h-9 w-9 items-center justify-center rounded-full bg-white/15 ring-1 ring-white/25">
              <IconMessageCircle className="h-5 w-5" />
            </span>
            <div>
              <h2 className="text-base font-semibold tracking-tight">Messagerie</h2>
              <p className="text-[11px] text-sky-100/90">10 dernières conversations</p>
            </div>
          </div>
          <div className="flex items-center gap-0.5">
            <Link
              href="/messagerie"
              className="rounded-full p-2 text-white/80 hover:bg-white/15 hover:text-white"
              title="Page complète"
              onClick={onClose}
            >
              <IconExternalLink className="h-4 w-4" />
            </Link>
            <button
              type="button"
              className="rounded-full p-2 text-white/80 hover:bg-white/15 hover:text-white"
              onClick={onClose}
              aria-label="Fermer"
            >
              <IconX className="h-4 w-4" />
            </button>
          </div>
        </div>

        <div className="relative mt-3">
          <IconSearch className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-sky-900/40" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Rechercher…"
            className="w-full rounded-full border-0 bg-white/95 py-2 pl-9 pr-3 text-sm text-slate-800 shadow-sm outline-none placeholder:text-slate-400 focus:ring-2 focus:ring-white/60"
          />
        </div>

        {myPresence && onSetManualStatus ? (
          <div className="relative mt-3">
            <MessagingStatusPicker myPresence={myPresence} onSetManual={onSetManualStatus} />
          </div>
        ) : null}

        <div className="relative mt-3 flex gap-1 rounded-full bg-black/15 p-1">
          {(
            [
              { id: "recent" as const, label: "Récents" },
              { id: "people" as const, label: "Personnel" },
              { id: "group" as const, label: "Groupe" },
            ] as const
          ).map((t) => (
            <button
              key={t.id}
              type="button"
              className={`flex-1 rounded-full px-2 py-1.5 text-xs font-medium transition ${
                tab === t.id
                  ? "bg-white text-sky-800 shadow-sm"
                  : "text-sky-50/90 hover:bg-white/10"
              }`}
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          ))}
        </div>
      </header>

      <div className="flex-1 overflow-y-auto bg-gradient-to-b from-slate-50/80 to-white">
        {searchResults.length > 0 && tab === "recent" ? (
          <section className="border-b border-slate-100 p-2">
            <p className="mb-1 px-2 text-[10px] font-semibold uppercase tracking-wide text-slate-400">
              Messages
            </p>
            {searchResults.map((r) => {
              const conv =
                allConversations.find((c) => c.id === r.conversationId) ??
                conversations.find((c) => c.id === r.conversationId);
              return (
                <button
                  key={r.messageId}
                  type="button"
                  className="mb-0.5 w-full rounded-xl px-2 py-2 text-left hover:bg-sky-50"
                  onClick={() => {
                    if (conv) onOpenConversation(conv);
                  }}
                >
                  <p className="truncate text-xs font-medium text-slate-700">
                    {conv?.title ?? "Conversation"}
                  </p>
                  <p className="truncate text-[11px] text-slate-400">{r.body}</p>
                </button>
              );
            })}
          </section>
        ) : null}

        {tab === "recent" ? (
          <section className="p-2">
            {conversations.length === 0 ? (
              <div className="px-3 py-10 text-center">
                <IconMessageCircle className="mx-auto mb-2 h-8 w-8 text-slate-300" />
                <p className="text-sm text-slate-500">Aucune conversation récente</p>
                <p className="mt-1 text-xs text-slate-400">
                  Onglet Personnel pour écrire à un collègue.
                </p>
              </div>
            ) : (
              conversations.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  className="mb-0.5 flex w-full items-center gap-3 rounded-xl px-2 py-2.5 text-left transition hover:bg-sky-50"
                  onClick={() => onOpenConversation(c)}
                >
                  <ConvAvatar conv={c} unread={c.unreadCount} presence={convPresence(c)} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <p className="truncate text-sm font-semibold text-slate-800">{c.title}</p>
                      {c.kind === "group" ? (
                        <span className="shrink-0 rounded bg-indigo-50 px-1 py-0.5 text-[9px] font-medium text-indigo-600">
                          Groupe
                        </span>
                      ) : null}
                    </div>
                    <p className="truncate text-[12px] text-slate-400">
                      {c.lastMessage?.body ||
                        (c.lastMessage?.type && c.lastMessage.type !== "text"
                          ? `📎 ${c.lastMessage.type}`
                          : "Dites bonjour…")}
                    </p>
                  </div>
                </button>
              ))
            )}
          </section>
        ) : null}

        {tab === "people" ? (
          <section className="p-2">
            {loadingUsers ? (
              <p className="px-2 py-6 text-center text-xs text-slate-400">Chargement…</p>
            ) : filteredUsers.length === 0 ? (
              <p className="px-2 py-6 text-center text-xs text-slate-400">Aucun résultat</p>
            ) : (
              filteredUsers.map((u) => (
                <button
                  key={u.id}
                  type="button"
                  className="mb-0.5 flex w-full items-center gap-3 rounded-xl px-2 py-2.5 text-left hover:bg-sky-50"
                  onClick={() => onStartWithPeer(u)}
                >
                  <PeerAvatar peer={u} presence={peerPresence(u.id)} />
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-slate-800">{u.name}</p>
                  </div>
                </button>
              ))
            )}
          </section>
        ) : null}

        {tab === "group" ? (
          <section className="space-y-3 p-3">
            <div className="flex items-start gap-2 rounded-xl bg-indigo-50/80 px-3 py-2.5 text-xs text-indigo-800">
              <IconUsers className="mt-0.5 h-4 w-4 shrink-0" />
              <p>Créez un groupe avec vos collègues (personnel uniquement).</p>
            </div>
            <div>
              <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wide text-slate-400">
                Nom du groupe
              </label>
              <input
                value={groupTitle}
                onChange={(e) => setGroupTitle(e.target.value)}
                placeholder="Ex. Vie scolaire, Projet bac…"
                className="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:border-sky-400"
              />
            </div>
            <div>
              <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wide text-slate-400">
                Membres ({groupMembers.length})
              </label>
              <div className="max-h-48 space-y-0.5 overflow-y-auto rounded-xl border border-slate-100 bg-white p-1">
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
                        onChange={() => {
                          setGroupMembers((prev) =>
                            checked ? prev.filter((id) => id !== u.id) : [...prev, u.id],
                          );
                        }}
                        className="rounded border-slate-300 text-sky-600 focus:ring-sky-500"
                      />
                      <PeerAvatar peer={u} presence={peerPresence(u.id)} />
                      <span className="truncate text-sm text-slate-800">{u.name}</span>
                    </label>
                  );
                })}
              </div>
            </div>
            <button
              type="button"
              disabled={creatingGroup || groupTitle.trim().length < 2 || groupMembers.length < 1}
              className="w-full rounded-full bg-gradient-to-r from-sky-600 to-indigo-600 py-2.5 text-sm font-semibold text-white shadow-md disabled:cursor-not-allowed disabled:opacity-40"
              onClick={() => void submitGroup()}
            >
              {creatingGroup ? "Création…" : "Créer le groupe"}
            </button>
          </section>
        ) : null}
      </div>
    </div>
  );
}
