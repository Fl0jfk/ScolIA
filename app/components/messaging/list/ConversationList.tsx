"use client";

import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import type {
  MessagingConversationDto,
  MessagingConversationPatchInput,
  MessagingPeer,
  MessagingPresenceStatus,
} from "@/app/lib/messaging/types";
import { IconPlus, IconSearch, IconUsers, IconX } from "@/app/components/messaging/MessagingIcons";
import ListAvatar from "./ListAvatar";
import { ListSkeleton } from "@/app/components/messaging/conversation/Skeletons";
import { IconChatEmpty } from "@/app/components/messaging/conversation/ExtraIcons";
import {
  IconAt,
  IconBell,
  IconBellOff,
  IconMailOpen,
  IconMailUnread,
  IconMore,
  IconPin,
  IconTrashList,
} from "./ListIcons";
import {
  conversationAvatarSource,
  conversationPreview,
  conversationTime,
  isUnread,
  peerLetter,
  sortConversations,
} from "./listHelpers";

type Filter = "all" | "unread" | "groups" | "pinned";
const FILTERS: Array<{ id: Filter; label: string }> = [
  { id: "all", label: "Tout" },
  { id: "unread", label: "Non lus" },
  { id: "groups", label: "Groupes" },
  { id: "pinned", label: "Épinglées" },
];

type SearchHit = {
  messageId: string;
  conversationId: string;
  body: string | null;
  createdAt: string;
};

type MenuState = {
  conversation: MessagingConversationDto;
  x: number;
  y: number;
  confirmDelete: boolean;
};

type Props = {
  conversations: MessagingConversationDto[];
  meId: string;
  selectedId: string | null;
  users: MessagingPeer[];
  statusOf: (id?: string | null) => MessagingPresenceStatus;
  statusForConversation: (c: MessagingConversationDto) => MessagingPresenceStatus;
  searchInputRef: RefObject<HTMLInputElement | null>;
  onSelect: (c: MessagingConversationDto) => void;
  onOpenConversationId: (id: string) => void;
  onStartWithPeer: (peer: MessagingPeer) => void;
  onPatch: (c: MessagingConversationDto, patch: MessagingConversationPatchInput) => void;
  onDelete: (c: MessagingConversationDto) => void;
  notify: (text: string) => void;
  /** Premier chargement de la liste en cours. */
  loading?: boolean;
  /** Ouvre la création de groupe. */
  onNewGroup?: () => void;
};

export default function ConversationList({
  conversations,
  meId,
  selectedId,
  users,
  statusOf,
  statusForConversation,
  searchInputRef,
  onSelect,
  onOpenConversationId,
  onStartWithPeer,
  onPatch,
  onDelete,
  notify,
  loading = false,
  onNewGroup,
}: Props) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [searching, setSearching] = useState(false);
  const pressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const q = query.trim().toLowerCase();

  useEffect(() => {
    setHits(null);
  }, [query]);

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", close);
    };
  }, [menu]);

  const sorted = useMemo(() => sortConversations(conversations), [conversations]);

  const visible = useMemo(() => {
    return sorted.filter((c) => {
      if (filter === "unread" && !isUnread(c)) return false;
      if (filter === "groups" && c.kind !== "group") return false;
      if (filter === "pinned" && !c.pinned) return false;
      if (
        q &&
        !c.title.toLowerCase().includes(q) &&
        !(c.peer?.email ?? "").toLowerCase().includes(q)
      )
        return false;
      return true;
    });
  }, [sorted, filter, q]);

  const pinned = filter === "all" && !q ? visible.filter((c) => c.pinned) : [];
  const others = filter === "all" && !q ? visible.filter((c) => !c.pinned) : visible;

  const filteredUsers = q
    ? users
        .filter((u) => u.id !== meId)
        .filter((u) => u.name.toLowerCase().includes(q) || u.email.toLowerCase().includes(q))
        .slice(0, 15)
    : [];

  const searchMessages = async () => {
    if (q.length < 2) return;
    setSearching(true);
    try {
      const res = await fetch(`/api/messaging/search?q=${encodeURIComponent(query.trim())}`, {
        cache: "no-store",
      });
      if (!res.ok) throw new Error();
      const data = (await res.json()) as { results?: SearchHit[] };
      setHits(Array.isArray(data.results) ? data.results : []);
    } catch {
      notify("Recherche impossible pour le moment.");
    } finally {
      setSearching(false);
    }
  };

  const openMenuAt = (c: MessagingConversationDto, x: number, y: number) => {
    const w = 260;
    const h = 250;
    setMenu({
      conversation: c,
      x: Math.max(8, Math.min(x, window.innerWidth - w - 8)),
      y: Math.max(8, Math.min(y, window.innerHeight - h - 8)),
      confirmDelete: false,
    });
  };

  const clearPress = () => {
    if (pressTimer.current) {
      clearTimeout(pressTimer.current);
      pressTimer.current = null;
    }
  };

  const renderItem = (c: MessagingConversationDto) => {
    const active = selectedId === c.id;
    const unread = isUnread(c);
    const av = conversationAvatarSource(c);
    return (
      <div
        key={c.id}
        className="group relative"
        onContextMenu={(e) => {
          e.preventDefault();
          openMenuAt(c, e.clientX, e.clientY);
        }}
        onTouchStart={(e) => {
          const t = e.touches[0];
          clearPress();
          pressTimer.current = setTimeout(() => openMenuAt(c, t.clientX, t.clientY), 500);
        }}
        onTouchEnd={clearPress}
        onTouchMove={clearPress}
        onTouchCancel={clearPress}
      >
        <button
          type="button"
          onClick={() => onSelect(c)}
          aria-current={active ? "true" : undefined}
          className={`relative flex w-full items-center gap-3 rounded-2xl px-3 py-2.5 text-left outline-none transition focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 ${
            active
              ? "bg-white shadow-[0_1px_2px_rgba(0,0,0,0.06)] ring-1 ring-black/5"
              : "hover:bg-white/80"
          }`}
        >
          {active ? (
            <span
              aria-hidden
              className="absolute bottom-2.5 left-0 top-2.5 w-[3px] rounded-full bg-[var(--dash-ink)]"
            />
          ) : null}
          <ListAvatar
            imageUrl={av.imageUrl}
            letter={av.letter}
            name={av.name}
            status={statusForConversation(c)}
          />
          <div className="min-w-0 flex-1">
            <div className="flex items-baseline justify-between gap-2">
              <p
                className={`truncate text-sm text-[var(--dash-ink)] ${
                  unread ? "font-bold" : "font-semibold"
                }`}
              >
                {c.title}
              </p>
              <span
                className={`shrink-0 text-[11px] tabular-nums ${
                  unread ? "font-semibold text-[var(--dash-ink)]" : "text-neutral-500"
                }`}
              >
                {conversationTime(c.lastMessageAt ?? c.lastMessage?.createdAt)}
              </span>
            </div>
            <div className="mt-0.5 flex h-5 items-center gap-1.5">
              <p
                className={`min-w-0 flex-1 truncate text-[12.5px] ${
                  unread ? "font-semibold text-[var(--dash-ink)]" : "text-neutral-500"
                }`}
              >
                {conversationPreview(c, meId)}
              </p>
              {c.muted ? <IconBellOff className="h-3 w-3 shrink-0 text-neutral-500" /> : null}
              {c.pinned && (filter !== "all" || q) ? (
                <IconPin className="h-3 w-3 shrink-0 text-neutral-500" />
              ) : null}
              {c.hasUnreadMention ? (
                <span
                  title="Vous avez été mentionné"
                  className="inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-[var(--dash-ink)] text-white"
                >
                  <IconAt className="h-2.5 w-2.5" />
                </span>
              ) : null}
              {unread ? (
                c.unreadCount > 0 ? (
                  <span
                    className={`min-w-[1.25rem] shrink-0 rounded-full px-1.5 text-center text-[11px] font-bold leading-5 ${
                      c.muted
                        ? "bg-neutral-300 text-neutral-700"
                        : "bg-[var(--dash-lime)] text-[var(--dash-ink)]"
                    }`}
                  >
                    {c.unreadCount > 99 ? "99+" : c.unreadCount}
                  </span>
                ) : (
                  <span className="h-2.5 w-2.5 shrink-0 rounded-full bg-[var(--dash-lime)] ring-1 ring-[var(--dash-ink)]/20" />
                )
              ) : null}
            </div>
          </div>
        </button>
        <button
          type="button"
          aria-label="Options de la conversation"
          onClick={(e) => {
            e.stopPropagation();
            const r = e.currentTarget.getBoundingClientRect();
            openMenuAt(c, r.right - 240, r.bottom + 4);
          }}
          className="absolute right-2 top-1/2 hidden h-7 w-7 -translate-y-1/2 items-center justify-center rounded-full bg-white text-neutral-600 shadow ring-1 ring-black/5 outline-none transition hover:text-[var(--dash-ink)] focus-visible:flex focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 group-hover:flex [@media(hover:none)]:hidden"
        >
          <IconMore className="h-3.5 w-3.5" />
        </button>
      </div>
    );
  };

  const titleOf = (id: string) => conversations.find((c) => c.id === id)?.title ?? "Conversation";

  const menuItem =
    "flex w-full items-center gap-2.5 rounded-xl px-3 py-2 text-left text-[13px] font-medium text-[var(--dash-ink)] transition hover:bg-[#eceeea]";

  return (
    <>
      <div className="space-y-2 border-b border-black/5 p-2.5">
        <div className="relative">
          <IconSearch className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-neutral-500" />
          <input
            ref={searchInputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setQuery("");
            }}
            placeholder="Rechercher (Ctrl+K)"
            aria-label="Rechercher une conversation ou un collègue"
            className="w-full rounded-2xl border border-black/6 bg-white/80 py-2 pl-9 pr-8 text-sm text-[var(--dash-ink)] outline-none transition placeholder:text-neutral-500 focus:border-[var(--dash-ink)] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/20"
          />
          {query ? (
            <button
              type="button"
              aria-label="Effacer la recherche"
              onClick={() => setQuery("")}
              className="absolute right-2.5 top-2.5 rounded text-neutral-500 outline-none hover:text-[var(--dash-ink)] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50"
            >
              <IconX className="h-4 w-4" />
            </button>
          ) : null}
        </div>
        <div className="flex flex-wrap gap-1.5">
          {FILTERS.map((f) => (
            <button
              key={f.id}
              type="button"
              onClick={() => setFilter(f.id)}
              aria-pressed={filter === f.id}
              className={`rounded-full px-2.5 py-1 text-[11px] font-semibold outline-none transition focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 ${
                filter === f.id
                  ? "bg-[var(--dash-ink)] text-white"
                  : "bg-white/70 text-[var(--dash-ink)] hover:bg-white"
              }`}
            >
              {f.label}
            </button>
          ))}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2 pt-1">
        {q ? (
          <button
            type="button"
            disabled={q.length < 2 || searching}
            onClick={() => void searchMessages()}
            className="mb-1 flex w-full items-center gap-2 rounded-2xl border border-dashed border-black/10 px-3 py-2 text-left text-xs font-semibold text-[var(--dash-ink)] transition hover:bg-white/70 disabled:opacity-50"
          >
            <IconSearch className="h-3.5 w-3.5" />
            {searching ? "Recherche…" : `Rechercher « ${query.trim()} » dans les messages`}
          </button>
        ) : null}

        {hits ? (
          <div className="mb-2">
            <p className="px-2 py-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--dash-mid)]">
              Dans les messages
            </p>
            {hits.length === 0 ? (
              <p className="px-3 py-2 text-xs text-neutral-500">Aucun message trouvé.</p>
            ) : (
              hits.slice(0, 20).map((h) => (
                <button
                  key={h.messageId}
                  type="button"
                  onClick={() => onOpenConversationId(h.conversationId)}
                  className="block w-full rounded-2xl px-3 py-2 text-left transition hover:bg-white/70"
                >
                  <span className="flex items-baseline justify-between gap-2">
                    <span className="truncate text-xs font-semibold text-[var(--dash-ink)]">
                      {titleOf(h.conversationId)}
                    </span>
                    <span className="shrink-0 text-[11px] text-neutral-500">
                      {conversationTime(h.createdAt)}
                    </span>
                  </span>
                  <span className="mt-0.5 line-clamp-2 text-[12px] text-neutral-500">{h.body}</span>
                </button>
              ))
            )}
          </div>
        ) : null}

        {filteredUsers.length > 0 ? (
          <div className="mb-2">
            <p className="px-2 py-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--dash-mid)]">
              Personnel
            </p>
            {filteredUsers.map((u) => (
              <button
                key={u.id}
                type="button"
                className="flex w-full items-center gap-2.5 rounded-2xl px-2.5 py-2 text-left transition hover:bg-white/70"
                onClick={() => {
                  setQuery("");
                  onStartWithPeer(u);
                }}
              >
                <ListAvatar
                  imageUrl={u.imageUrl}
                  letter={peerLetter(u)}
                  name={u.name}
                  status={statusOf(u.id)}
                  size="h-9 w-9"
                />
                <span className="truncate text-sm font-medium text-[var(--dash-ink)]">{u.name}</span>
              </button>
            ))}
          </div>
        ) : null}

        {pinned.length > 0 ? (
          <>
            <p className="flex items-center gap-1 px-2 py-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--dash-mid)]">
              <IconPin className="h-3 w-3" /> Épinglées
            </p>
            {pinned.map(renderItem)}
          </>
        ) : null}

        {loading && conversations.length === 0 ? (
          <ListSkeleton />
        ) : conversations.length === 0 && !q ? (
          <div className="flex flex-col items-center px-4 py-10 text-center">
            <span className="flex h-14 w-14 items-center justify-center rounded-full bg-white text-[var(--dash-mid)] shadow-sm">
              <IconChatEmpty className="h-7 w-7" />
            </span>
            <p className="mt-3 text-sm font-semibold text-[var(--dash-ink)]">Aucune conversation</p>
            <p className="mt-1 text-xs leading-relaxed text-[var(--dash-mid)]">
              Écrivez à un collègue ou créez un groupe pour démarrer.
            </p>
            <div className="mt-4 flex flex-wrap justify-center gap-2">
              <button
                type="button"
                onClick={() => {
                  searchInputRef.current?.focus();
                }}
                className="inline-flex items-center gap-1.5 rounded-2xl bg-[var(--dash-ink)] px-3 py-2 text-xs font-semibold text-white outline-none transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-[var(--dash-lime)] focus-visible:ring-offset-1"
              >
                <IconSearch className="h-3.5 w-3.5" />
                Chercher un collègue
              </button>
              {onNewGroup ? (
                <button
                  type="button"
                  onClick={onNewGroup}
                  className="inline-flex items-center gap-1.5 rounded-2xl bg-white px-3 py-2 text-xs font-semibold text-[var(--dash-ink)] shadow-sm outline-none transition hover:bg-white/70 focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50"
                >
                  <IconPlus className="h-3.5 w-3.5" />
                  <IconUsers className="h-3.5 w-3.5" />
                  Créer un groupe
                </button>
              ) : null}
            </div>
          </div>
        ) : (
          <>
            {others.length > 0 || pinned.length > 0 ? (
              <p className="px-2 py-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--dash-mid)]">
                {pinned.length > 0 ? "Récentes" : "Conversations"}
              </p>
            ) : null}
            {others.length === 0 && pinned.length === 0 ? (
              <p className="px-3 py-6 text-center text-xs leading-relaxed text-neutral-600">
                {filter !== "all" && !q
                  ? "Rien dans ce filtre pour le moment."
                  : "Aucune conversation ne correspond."}
              </p>
            ) : (
              others.map(renderItem)
            )}
          </>
        )}
      </div>

      {menu ? (
        <div
          className="fixed inset-0 z-[160]"
          onClick={() => setMenu(null)}
          onContextMenu={(e) => {
            e.preventDefault();
            setMenu(null);
          }}
        >
          <div
            role="menu"
            onClick={(e) => e.stopPropagation()}
            style={{ left: menu.x, top: menu.y }}
            className="absolute w-[260px] rounded-2xl border border-black/8 bg-white p-1.5 shadow-2xl"
          >
            {menu.confirmDelete ? (
              <div className="p-2">
                <p className="text-sm font-semibold text-[var(--dash-ink)]">
                  Supprimer la conversation ?
                </p>
                <p className="mt-1 text-xs leading-relaxed text-neutral-500">
                  Elle disparaîtra de votre liste. Les autres participants la conservent. Elle
                  réapparaîtra si quelqu’un vous écrit.
                </p>
                <div className="mt-3 flex gap-2">
                  <button
                    type="button"
                    className="flex-1 rounded-xl bg-[#eceeea] py-2 text-xs font-semibold text-[var(--dash-ink)]"
                    onClick={() => setMenu(null)}
                  >
                    Annuler
                  </button>
                  <button
                    type="button"
                    className="flex-1 rounded-xl bg-red-600 py-2 text-xs font-semibold text-white"
                    onClick={() => {
                      onDelete(menu.conversation);
                      setMenu(null);
                    }}
                  >
                    Supprimer
                  </button>
                </div>
              </div>
            ) : (
              <>
                <button
                  type="button"
                  role="menuitem"
                  className={menuItem}
                  onClick={() => {
                    onPatch(menu.conversation, { pinned: !menu.conversation.pinned });
                    setMenu(null);
                  }}
                >
                  <IconPin className="h-4 w-4" />
                  {menu.conversation.pinned ? "Désépingler" : "Épingler"}
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className={menuItem}
                  onClick={() => {
                    onPatch(menu.conversation, { markedUnread: !isUnread(menu.conversation) });
                    setMenu(null);
                  }}
                >
                  {isUnread(menu.conversation) ? (
                    <IconMailOpen className="h-4 w-4" />
                  ) : (
                    <IconMailUnread className="h-4 w-4" />
                  )}
                  {isUnread(menu.conversation) ? "Marquer comme lu" : "Marquer comme non lu"}
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className={menuItem}
                  onClick={() => {
                    onPatch(menu.conversation, { muted: !menu.conversation.muted });
                    setMenu(null);
                  }}
                >
                  {menu.conversation.muted ? (
                    <IconBell className="h-4 w-4" />
                  ) : (
                    <IconBellOff className="h-4 w-4" />
                  )}
                  {menu.conversation.muted
                    ? "Réactiver les notifications"
                    : "Couper les notifications"}
                </button>
                <div className="my-1 h-px bg-black/5" />
                <button
                  type="button"
                  role="menuitem"
                  className={`${menuItem} !text-red-600`}
                  onClick={() => setMenu({ ...menu, confirmDelete: true })}
                >
                  <IconTrashList className="h-4 w-4" />
                  Supprimer la conversation
                </button>
              </>
            )}
          </div>
        </div>
      ) : null}
    </>
  );
}
