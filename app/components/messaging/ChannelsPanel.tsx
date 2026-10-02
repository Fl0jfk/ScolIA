"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import Image from "next/image";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import {
  IconHash,
  IconLock,
  IconPlus,
  IconTrash,
  IconUsers,
  IconX,
} from "@/app/components/messaging/MessagingIcons";

type Channel = {
  id: string;
  name: string;
  type: "public" | "restricted" | "private";
  roleRequired?: string;
  members?: string[];
  creatorId?: string;
};

type ChannelMessage = {
  id: number;
  channel: string;
  content: string;
  createdAt: string;
  authorName: string;
  authorId: string | null;
  avatar: string | null;
};

type DirectoryUser = {
  id: string;
  name: string;
  avatar: string;
};

const AUTHORIZED_ADMINS = [
  "HACQUEVILLE-MATHI",
  "DONA",
  "DUMOUCHEL",
  "PLANTEC",
  "LEBLOND",
];

type Props = {
  /** Mode embarqué dans la messagerie (pas de chrome page). */
  embedded?: boolean;
  className?: string;
};

/**
 * Salons (channels) — même UI bento que la messagerie, réutilisable en page
 * dédiée (élèves) ou onglet de la messagerie (personnel).
 */
export default function ChannelsPanel({ embedded = false, className = "" }: Props) {
  const { user, isLoaded } = useSessionUser();
  const [channels, setChannels] = useState<Channel[]>([]);
  const [activeChannel, setActiveChannel] = useState("general");
  const [messages, setMessages] = useState<ChannelMessage[]>([]);
  const [text, setText] = useState("");
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [showMembersModal, setShowMembersModal] = useState(false);
  const [newChanName, setNewChanName] = useState("");
  const [allUsers, setAllUsers] = useState<DirectoryUser[]>([]);
  const [selectedMembers, setSelectedMembers] = useState<string[]>([]);
  const [isPrivate, setIsPrivate] = useState(false);
  const [nameError, setNameError] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [unreadChannels, setUnreadChannels] = useState<string[]>([]);
  const [listOpen, setListOpen] = useState(false);
  const lastSeenMessageId = useRef<Record<string, number>>({});
  const scrollRef = useRef<HTMLDivElement>(null);
  const roles = rolesFromUserLike(user);
  const isAdminByName = useMemo(() => {
    if (!user?.fullName) return false;
    return AUTHORIZED_ADMINS.includes(user.fullName);
  }, [user]);
  const sortedUsers = useMemo(() => {
    return [...allUsers].sort((a, b) => {
      const nameA = a.name.split(" ").pop() || "";
      const nameB = b.name.split(" ").pop() || "";
      return nameA.localeCompare(nameB);
    });
  }, [allUsers]);

  useEffect(() => {
    if (user?.id) {
      const saved = localStorage.getItem(`lastSeen_${user.id}`);
      if (saved) {
        try {
          lastSeenMessageId.current = JSON.parse(saved) as Record<string, number>;
        } catch {
          lastSeenMessageId.current = {};
        }
      }
    }
  }, [user?.id]);

  const fetchChannels = async () => {
    try {
      const res = await fetch("/api/channels");
      const data = (await res.json()) as Channel[];
      const accessible = data.filter((c) => {
        if (c.type === "public") return true;
        if (c.type === "restricted" && c.roleRequired && roles.includes(c.roleRequired)) {
          return true;
        }
        if (c.type === "private" && c.members?.includes(user?.id || "")) return true;
        return false;
      });
      setChannels(accessible);
      if (accessible.length > 0 && activeChannel === "general") {
        const generalChan = accessible.find((c) => c.name.toLowerCase() === "general");
        if (generalChan) setActiveChannel(generalChan.id);
        else setActiveChannel(accessible[0]?.id ?? "general");
      }
    } catch (err) {
      console.error("Erreur channels:", err);
    }
  };

  const fetchProfs = async () => {
    try {
      const res = await fetch("/api/channels/users/list");
      const data = (await res.json()) as DirectoryUser[];
      setAllUsers(data.filter((p) => p.id !== user?.id));
    } catch (err) {
      console.error("Erreur liste profs:", err);
    }
  };

  const fetchMessages = async (isInitialLoad = false) => {
    if (isInitialLoad) setLoading(true);
    try {
      const res = await fetch("/api/channels/messages");
      const allMessages = (await res.json()) as ChannelMessage[];
      const filtered = allMessages.filter((m) => m.channel === activeChannel);
      setMessages(filtered);
      const newUnreads: string[] = [];
      channels.forEach((chan) => {
        const chanMessages = allMessages.filter((m) => m.channel === chan.id);
        const lastMsgInChan = chanMessages[chanMessages.length - 1];
        if (chan.id === activeChannel) {
          if (lastMsgInChan) {
            lastSeenMessageId.current[chan.id] = lastMsgInChan.id;
            localStorage.setItem(
              `lastSeen_${user?.id}`,
              JSON.stringify(lastSeenMessageId.current),
            );
          }
          return;
        }
        const lastSeenId = lastSeenMessageId.current[chan.id] || 0;
        if (lastMsgInChan && lastMsgInChan.id > lastSeenId) {
          newUnreads.push(chan.id);
        }
      });
      setUnreadChannels(newUnreads);
    } catch (err) {
      console.error("Erreur messages:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isLoaded && user) {
      void fetchChannels();
      void fetchProfs();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isLoaded, user?.id]);

  useEffect(() => {
    if (isLoaded && activeChannel && channels.length > 0) {
      void fetchMessages(true);
      const interval = setInterval(() => void fetchMessages(false), 5000);
      return () => clearInterval(interval);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeChannel, isLoaded, channels.length]);

  useEffect(() => {
    if (scrollRef.current && !loading) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, loading]);

  const handleSend = async (anonymous: boolean, overrideContent?: string) => {
    const contentToSend = overrideContent || text;
    if (!contentToSend.trim()) return;
    const res = await fetch("/api/channels/messages", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        content: contentToSend,
        channel: activeChannel,
        isAnonymous: anonymous,
      }),
    });
    if (res.ok) {
      if (!overrideContent) setText("");
      void fetchMessages(false);
    }
  };

  const handleDeleteMessage = async (messageId: number) => {
    if (!confirm("Supprimer ce message ?")) return;
    try {
      const res = await fetch(`/api/channels/messages?id=${messageId}`, {
        method: "DELETE",
      });
      if (res.ok) void fetchMessages(false);
    } catch (err) {
      console.error("Erreur suppression:", err);
    }
  };

  const handleDeleteChannel = async () => {
    const current = channels.find((c) => c.id === activeChannel);
    if (!current) return;
    const firstConfirm = confirm(`Supprimer #${current.name} ?`);
    if (firstConfirm && prompt("Écrire SUPPRIMER :") === "SUPPRIMER") {
      try {
        const res = await fetch(`/api/channels?id=${current.id}`, { method: "DELETE" });
        if (res.ok) {
          setActiveChannel("general");
          void fetchChannels();
        }
      } catch (err) {
        console.error(err);
      }
    }
  };

  const handleUpdateMembers = async (newMembers: string[]) => {
    try {
      const res = await fetch("/api/channels", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          channelId: activeChannel,
          members: newMembers,
        }),
      });
      if (res.ok) void fetchChannels();
    } catch (err) {
      console.error("Erreur mise à jour membres:", err);
    }
  };

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setIsUploading(true);
    const formData = new FormData();
    formData.append("file", file);
    try {
      const res = await fetch("/api/channels/upload", { method: "POST", body: formData });
      const data = (await res.json()) as { url?: string; name?: string };
      if (data.url) {
        const content = file.type.startsWith("image/")
          ? `[IMG]${data.url}`
          : `[DOC]${data.name}|${data.url}`;
        await handleSend(false, content);
      }
    } catch (err) {
      console.log(err);
      alert("Erreur envoi");
    } finally {
      setIsUploading(false);
    }
  };

  const toggleMemberCreation = (id: string) => {
    setSelectedMembers((prev) =>
      prev.includes(id) ? prev.filter((m) => m !== id) : [...prev, id],
    );
  };

  const toggleMemberUpdate = (id: string) => {
    const currentChannel = channels.find((c) => c.id === activeChannel);
    if (!currentChannel) return;
    const currentList = currentChannel.members || [];
    const newList = currentList.includes(id)
      ? currentList.filter((m) => m !== id)
      : [...currentList, id];
    if (currentChannel.creatorId && !newList.includes(currentChannel.creatorId)) {
      newList.push(currentChannel.creatorId);
    }
    void handleUpdateMembers(newList);
  };

  const handleCreateChannel = async () => {
    setNameError(false);
    if (!newChanName.trim()) {
      setNameError(true);
      return;
    }
    try {
      const res = await fetch("/api/channels", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: newChanName.trim(),
          type: isPrivate ? "private" : "public",
          members: isPrivate ? [...selectedMembers, user?.id] : [],
          creatorId: user?.id,
        }),
      });
      if (res.ok) {
        setNewChanName("");
        setSelectedMembers([]);
        setIsPrivate(false);
        setShowModal(false);
        void fetchChannels();
      }
    } catch (err) {
      console.error(err);
    }
  };

  if (!isLoaded) {
    return (
      <div className={`flex h-full items-center justify-center text-sm text-neutral-400 ${className}`}>
        Chargement…
      </div>
    );
  }

  const currentChannel = channels.find((c) => c.id === activeChannel);
  const isCreator = currentChannel?.creatorId === user?.id;

  return (
    <div
      className={`relative flex min-h-0 flex-1 gap-3 ${className}`}
      data-tour={embedded ? undefined : "channels-list"}
    >
      <aside
        data-tour="channels-list"
        className={`
          ${listOpen ? "translate-x-0" : "-translate-x-[110%]"}
          md:translate-x-0 fixed md:static inset-y-4 left-4 z-40
          flex w-[15.5rem] flex-col overflow-hidden rounded-[1.75rem]
          border border-black/6 bg-[#eceeea] shadow-[0_1px_0_rgba(0,0,0,0.03)]
          transition-transform duration-300 ease-out md:inset-auto
        `}
      >
        <div className="flex items-center justify-between gap-2 px-3.5 py-3">
          <div className="min-w-0">
            <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--dash-mid)]">
              Salons
            </p>
            <p className="truncate text-sm font-semibold text-[var(--dash-ink)]">
              {channels.length} salon{channels.length > 1 ? "s" : ""}
            </p>
          </div>
          <button
            type="button"
            onClick={() => setShowModal(true)}
            className="flex h-8 w-8 items-center justify-center rounded-2xl bg-[var(--dash-ink)] text-white transition hover:opacity-90"
            title="Nouveau salon"
          >
            <IconPlus className="h-4 w-4" />
          </button>
        </div>
        <div className="min-h-0 flex-1 space-y-1 overflow-y-auto px-2.5 pb-3">
          {channels.map((c) => {
            const active = activeChannel === c.id;
            return (
              <button
                key={c.id}
                type="button"
                onClick={() => {
                  setActiveChannel(c.id);
                  setUnreadChannels((prev) => prev.filter((id) => id !== c.id));
                  setListOpen(false);
                }}
                className={`flex w-full items-center justify-between gap-2 rounded-2xl px-3 py-2.5 text-left transition ${
                  active
                    ? "bg-[var(--dash-ink)] text-white shadow-sm"
                    : "text-[var(--dash-ink)] hover:bg-white/70"
                }`}
              >
                <span className="flex min-w-0 items-center gap-2">
                  {c.type === "private" ? (
                    <IconLock className={`h-3.5 w-3.5 shrink-0 ${active ? "text-white/70" : "text-[var(--dash-mid)]"}`} />
                  ) : (
                    <IconHash className={`h-3.5 w-3.5 shrink-0 ${active ? "text-white/70" : "text-[var(--dash-mid)]"}`} />
                  )}
                  <span className="truncate text-sm font-semibold">{c.name}</span>
                </span>
                {unreadChannels.includes(c.id) ? (
                  <span className="h-2 w-2 shrink-0 rounded-full bg-rose-500" />
                ) : null}
              </button>
            );
          })}
        </div>
      </aside>

      {listOpen ? (
        <div
          className="fixed inset-0 z-30 bg-black/20 md:hidden"
          onClick={() => setListOpen(false)}
          aria-hidden
        />
      ) : null}

      <section className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-[1.75rem] border border-black/6 bg-white/80 shadow-[0_1px_0_rgba(0,0,0,0.03)] backdrop-blur-xl">
        <header className="flex items-center justify-between gap-3 border-b border-black/5 px-4 py-3 sm:px-5">
          <div className="flex min-w-0 items-center gap-3">
            <button
              type="button"
              onClick={() => setListOpen(true)}
              className="flex h-10 w-10 items-center justify-center rounded-2xl bg-[#eceeea] text-[var(--dash-ink)] md:hidden"
              aria-label="Liste des salons"
            >
              »
            </button>
            <div className="min-w-0">
              <h2 className="truncate text-base font-semibold tracking-tight text-[var(--dash-ink)]">
                #{currentChannel?.name || activeChannel}
              </h2>
              <p className="text-[11px] font-medium text-[var(--dash-mid)]">
                Discussion entre collègues
              </p>
            </div>
          </div>
          {isCreator && activeChannel !== "general" ? (
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => setShowMembersModal(true)}
                className="rounded-2xl p-2 text-[var(--dash-mid)] transition hover:bg-black/[0.04] hover:text-[var(--dash-ink)]"
                title="Gérer les membres"
              >
                <IconUsers className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={() => void handleDeleteChannel()}
                className="rounded-2xl p-2 text-[var(--dash-mid)] transition hover:bg-rose-50 hover:text-rose-600"
                title="Supprimer le salon"
              >
                <IconTrash className="h-4 w-4" />
              </button>
            </div>
          ) : null}
        </header>

        <div
          ref={scrollRef}
          data-tour="channels-messages"
          className="min-h-0 flex-1 space-y-4 overflow-y-auto px-4 py-4 sm:px-5"
        >
          {loading ? (
            <p className="text-center text-xs text-neutral-400">Chargement…</p>
          ) : messages.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-1 text-neutral-400">
              <p className="text-sm">Aucun message ici.</p>
            </div>
          ) : (
            messages.map((m) => {
              const isMe = m.authorId === user?.id;
              const canDelete = isMe || isAdminByName;
              return (
                <div
                  key={m.id}
                  className={`group flex ${isMe ? "justify-end" : "justify-start"}`}
                >
                  <div
                    className={`flex max-w-[85%] flex-col md:max-w-[70%] ${
                      isMe ? "items-end" : "items-start"
                    }`}
                  >
                    <div className="mb-1 flex items-center gap-2 px-1">
                      {canDelete ? (
                        <button
                          type="button"
                          onClick={() => void handleDeleteMessage(m.id)}
                          className="text-[10px] text-rose-400 opacity-0 transition group-hover:opacity-100 hover:text-rose-600"
                        >
                          Supprimer
                        </button>
                      ) : null}
                      <span className="truncate text-[10px] text-neutral-400">
                        {m.authorName} ·{" "}
                        {new Date(m.createdAt).toLocaleTimeString([], {
                          hour: "2-digit",
                          minute: "2-digit",
                        })}
                      </span>
                    </div>
                    <div
                      className={`w-full break-words rounded-[1.25rem] px-3.5 py-2.5 text-sm shadow-[0_1px_0_rgba(0,0,0,0.03)] ${
                        isMe
                          ? "rounded-br-md bg-[var(--dash-ink)] text-white"
                          : "rounded-bl-md border border-black/6 bg-white text-[var(--dash-ink)]"
                      }`}
                    >
                      {m.content.startsWith("[IMG]") ? (
                        <Image
                          src={m.content.replace("[IMG]", "")}
                          className="max-w-full cursor-pointer rounded-xl"
                          alt="Partage"
                          width={200}
                          height={200}
                          onClick={() =>
                            window.open(m.content.replace("[IMG]", ""), "_blank")
                          }
                        />
                      ) : m.content.startsWith("[DOC]") ? (
                        <a
                          href={m.content.split("|")[1]}
                          target="_blank"
                          rel="noreferrer"
                          className={`flex items-center gap-2 p-1 font-medium underline ${
                            isMe ? "text-white/85" : "text-[var(--dash-ink)]"
                          }`}
                        >
                          📄 {m.content.split("|")[0]?.replace("[DOC]", "")}
                        </a>
                      ) : (
                        m.content
                      )}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        <div
          data-tour="channels-compose"
          className="border-t border-black/5 bg-[#eceeea]/40 p-3 sm:p-4"
        >
          <div className="rounded-[1.25rem] border border-black/6 bg-white p-2">
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              className="h-16 w-full resize-none border-none bg-transparent p-3 text-sm text-[var(--dash-ink)] outline-none focus:ring-0 md:h-20"
              placeholder={
                isUploading
                  ? "Envoi…"
                  : `Message dans #${currentChannel?.name || activeChannel}…`
              }
              disabled={isUploading}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  void handleSend(false);
                }
              }}
            />
            <div className="flex items-center justify-between p-1">
              <label className="cursor-pointer rounded-xl p-2 transition hover:bg-black/[0.04]">
                <input
                  type="file"
                  className="hidden"
                  onChange={(e) => void handleFileUpload(e)}
                  disabled={isUploading}
                />
                <span className="text-lg leading-none">📎</span>
              </label>
              <div className="flex gap-1.5">
                <button
                  type="button"
                  onClick={() => void handleSend(true)}
                  className="rounded-2xl px-3 py-2 text-[11px] font-semibold text-[var(--dash-mid)] transition hover:bg-black/[0.04]"
                  disabled={isUploading}
                >
                  Anonyme
                </button>
                <button
                  type="button"
                  onClick={() => void handleSend(false)}
                  className="rounded-2xl bg-[var(--dash-ink)] px-4 py-2 text-[11px] font-semibold text-white transition hover:opacity-90 disabled:opacity-40"
                  disabled={isUploading}
                >
                  Envoyer
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      {showModal ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm">
          <div className="flex max-h-[90vh] w-full max-w-[450px] flex-col overflow-hidden rounded-[1.75rem] border border-black/6 bg-white p-6 shadow-2xl md:p-8">
            <div className="mb-4 flex items-center justify-between">
              <h3 className="text-lg font-semibold text-[var(--dash-ink)]">Nouveau salon</h3>
              <button
                type="button"
                onClick={() => {
                  setShowModal(false);
                  setNameError(false);
                }}
                className="rounded-2xl p-1.5 text-neutral-400 hover:bg-black/[0.04]"
              >
                <IconX className="h-4 w-4" />
              </button>
            </div>
            <div className="mb-4">
              <input
                autoFocus
                className={`w-full rounded-2xl border p-4 outline-none transition ${
                  nameError
                    ? "border-rose-500 bg-rose-50"
                    : "border-black/8 bg-[#eceeea]/60 focus:border-[var(--dash-ink)]"
                }`}
                placeholder="Nom du salon…"
                value={newChanName}
                onChange={(e) => {
                  setNewChanName(e.target.value);
                  if (nameError) setNameError(false);
                }}
              />
              {nameError ? (
                <p className="ml-2 mt-1 text-xs text-rose-500">Le nom est obligatoire</p>
              ) : null}
            </div>
            <div className="mb-4 flex items-center gap-2 px-1">
              <input
                type="checkbox"
                id="private-salon"
                checked={isPrivate}
                onChange={(e) => setIsPrivate(e.target.checked)}
                className="h-4 w-4 rounded"
              />
              <label
                htmlFor="private-salon"
                className="select-none text-sm font-medium text-[var(--dash-ink)]"
              >
                Salon privé (membres sélectionnés)
              </label>
            </div>
            {isPrivate ? (
              <div className="mb-6 flex-1 overflow-y-auto rounded-2xl border border-black/6 bg-[#eceeea]/50 p-4">
                <p className="mb-3 text-[10px] font-bold uppercase tracking-wider text-[var(--dash-mid)]">
                  Sélectionner les utilisateurs
                </p>
                <div className="space-y-1">
                  {sortedUsers.map((u) => {
                    const checked = selectedMembers.includes(u.id);
                    return (
                      <button
                        key={u.id}
                        type="button"
                        onClick={() => toggleMemberCreation(u.id)}
                        className={`flex w-full items-center gap-3 rounded-xl p-2 text-left transition ${
                          checked ? "bg-white" : "hover:bg-white/70"
                        }`}
                      >
                        <Image
                          src={u.avatar}
                          className="h-8 w-8 rounded-full border border-black/6"
                          width={32}
                          height={32}
                          alt=""
                        />
                        <span className="flex-1 truncate text-sm">{u.name}</span>
                        <span
                          className={`flex h-5 w-5 items-center justify-center rounded-full border ${
                            checked
                              ? "border-[var(--dash-ink)] bg-[var(--dash-ink)] text-white"
                              : "border-neutral-300"
                          }`}
                        >
                          {checked ? "✓" : ""}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            ) : null}
            <div className="mt-auto flex gap-3">
              <button
                type="button"
                onClick={() => {
                  setShowModal(false);
                  setNameError(false);
                }}
                className="flex-1 rounded-2xl py-3 font-semibold text-[var(--dash-mid)] transition hover:bg-black/[0.04]"
              >
                Annuler
              </button>
              <button
                type="button"
                onClick={() => void handleCreateChannel()}
                className="flex-1 rounded-2xl bg-[var(--dash-ink)] py-3 font-semibold text-white transition hover:opacity-90"
              >
                Créer
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {showMembersModal && currentChannel ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm">
          <div className="flex max-h-[90vh] w-full max-w-[450px] flex-col overflow-hidden rounded-[1.75rem] border border-black/6 bg-white p-6 shadow-2xl md:p-8">
            <div className="mb-6 flex items-center justify-between">
              <h3 className="text-lg font-semibold text-[var(--dash-ink)]">Membres du salon</h3>
              <button
                type="button"
                onClick={() => setShowMembersModal(false)}
                className="rounded-2xl p-1.5 text-neutral-400 hover:bg-black/[0.04]"
              >
                <IconX className="h-4 w-4" />
              </button>
            </div>
            <div className="mb-6 flex-1 space-y-1 overflow-y-auto pr-1">
              <p className="mb-3 text-[10px] font-bold uppercase tracking-wider text-[var(--dash-mid)]">
                Gérer l&apos;accès
              </p>
              {sortedUsers.map((u) => {
                const isMember = currentChannel.members?.includes(u.id);
                return (
                  <button
                    key={u.id}
                    type="button"
                    onClick={() => toggleMemberUpdate(u.id)}
                    className={`flex w-full items-center gap-3 rounded-2xl border p-3 text-left transition ${
                      isMember
                        ? "border-black/8 bg-[#eceeea]"
                        : "border-transparent bg-[#eceeea]/40 hover:border-black/6 hover:bg-white"
                    }`}
                  >
                    <Image
                      src={u.avatar}
                      width={36}
                      height={36}
                      className="h-9 w-9 rounded-full border border-black/6"
                      alt=""
                    />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-semibold text-[var(--dash-ink)]">
                        {u.name}
                      </p>
                      <p className="text-[10px] text-[var(--dash-mid)]">
                        {isMember ? "A accès au salon" : "Pas d'accès"}
                      </p>
                    </div>
                    <div
                      className={`relative h-6 w-10 rounded-full transition-colors ${
                        isMember ? "bg-[var(--dash-ink)]" : "bg-neutral-300"
                      }`}
                    >
                      <div
                        className={`absolute top-1 h-4 w-4 rounded-full bg-white transition-all ${
                          isMember ? "left-5" : "left-1"
                        }`}
                      />
                    </div>
                  </button>
                );
              })}
            </div>
            <button
              type="button"
              onClick={() => setShowMembersModal(false)}
              className="w-full rounded-2xl bg-[var(--dash-ink)] py-3.5 font-bold text-white transition hover:opacity-90"
            >
              Terminer
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
