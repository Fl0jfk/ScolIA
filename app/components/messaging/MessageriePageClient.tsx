"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useAppUser, useSessionUser } from "@/app/hooks/useAppUser";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import type {
  MessagingConversationDetailDto,
  MessagingConversationDto,
  MessagingConversationPatchInput,
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
import MessagingStatusPicker from "@/app/components/messaging/MessagingStatusPicker";
import { useMessagingPresence } from "@/app/components/messaging/useMessagingPresence";
import ChannelsPanel from "@/app/components/messaging/ChannelsPanel";
import {
  IconMessageCircle,
  IconUsers,
  IconHash,
  IconPlus,
} from "@/app/components/messaging/MessagingIcons";
import ConversationList from "@/app/components/messaging/list/ConversationList";
import ConversationDetailsDrawer from "@/app/components/messaging/list/ConversationDetailsDrawer";
import { useToast } from "@/app/components/messaging/list/useToast";
import { isUnread } from "@/app/components/messaging/list/listHelpers";

type Section = "messages" | "salons";
type ConversationsState = ReturnType<typeof useMessagingConversations>;

function parseSection(raw: string | null): Section {
  return raw === "salons" ? "salons" : "messages";
}

export default function MessageriePageClient() {
  const { user, isLoaded, isSignedIn } = useSessionUser();
  const { user: authUser } = useAppUser();
  const enabled = Boolean(isLoaded && isSignedIn && user);
  const convState = useMessagingConversations(enabled);
  // Identifiant Better-Auth (celui que le serveur compare). Jamais businessUserId.
  const currentUserId = convState.meId ?? authUser?.id ?? "";

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
      <MessageriePageBody
        currentUserId={currentUserId}
        enabled={enabled}
        convState={convState}
      />
    </MessagingCallProvider>
  );
}

function MessageriePageBody({
  currentUserId,
  enabled,
  convState,
}: {
  currentUserId: string;
  enabled: boolean;
  convState: ConversationsState;
}) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const section = parseSection(searchParams.get("section"));
  const cParam = searchParams.get("c");

  const { handleSseEvent, startCall, call } = useMessagingCall();
  const callBusy = Boolean(call && call.phase !== "idle");
  const { conversations, setConversations, refresh, totalUnread } = convState;
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
  const { toast, toastNode } = useToast();

  const [selectedId, setSelectedId] = useState<string | null>(cParam);
  const selectedIdRef = useRef<string | null>(cParam);
  const [mobileOpen, setMobileOpen] = useState(Boolean(cParam));
  const [detail, setDetail] = useState<MessagingConversationDetailDto | null>(null);
  const [detailTick, setDetailTick] = useState(0);
  const [showDetails, setShowDetails] = useState(false);
  const [users, setUsers] = useState<MessagingPeer[]>([]);
  const [forwardMessage, setForwardMessage] = useState<MessagingMessageDto | null>(null);
  const [showNewGroup, setShowNewGroup] = useState(false);
  const [groupTitle, setGroupTitle] = useState("");
  const [groupMembers, setGroupMembers] = useState<string[]>([]);
  const [creatingGroup, setCreatingGroup] = useState(false);
  const searchInputRef = useRef<HTMLInputElement | null>(null);

  const setSection = (next: Section) => {
    const params = new URLSearchParams(searchParams.toString());
    if (next === "messages") params.delete("section");
    else params.set("section", "salons");
    const qs = params.toString();
    router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
  };

  const writeUrl = useCallback(
    (id: string | null) => {
      const params = new URLSearchParams(window.location.search);
      if (id) params.set("c", id);
      else params.delete("c");
      const qs = params.toString();
      router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
    },
    [pathname, router],
  );

  const selectConv = useCallback(
    (id: string | null) => {
      selectedIdRef.current = id;
      setSelectedId(id);
      setMobileOpen(Boolean(id));
      setShowDetails(false);
      writeUrl(id);
    },
    [writeUrl],
  );

  // Lien profond : /messagerie?c=<id> (notifications) ou navigation arrière.
  useEffect(() => {
    if (cParam && cParam !== selectedIdRef.current) {
      selectedIdRef.current = cParam;
      setSelectedId(cParam);
      setMobileOpen(true);
      setShowDetails(false);
    }
  }, [cParam]);

  useEffect(() => {
    if (!enabled) return;
    fetch("/api/messaging/users", { cache: "no-store" })
      .then((r) => r.json())
      .then((data: { users?: MessagingPeer[] }) => {
        setUsers(Array.isArray(data.users) ? data.users : []);
      })
      .catch(() => setUsers([]));
  }, [enabled]);

  // Desktop : ouvre la première conversation par défaut (pas sur mobile, où la liste reste visible).
  useEffect(() => {
    if (selectedIdRef.current || cParam || conversations.length === 0) return;
    if (typeof window !== "undefined" && window.matchMedia("(min-width: 768px)").matches) {
      const first = conversations[0];
      selectedIdRef.current = first.id;
      setSelectedId(first.id);
    }
  }, [conversations, cParam]);

  // Détail (membres, messages épinglés) : à l'ouverture et sur conversation_updated.
  useEffect(() => {
    if (!enabled || !selectedId) {
      setDetail(null);
      return;
    }
    let cancelled = false;
    fetch(`/api/messaging/conversations/${selectedId}`, { cache: "no-store" })
      .then((r) => (r.ok ? r.json() : null))
      .then((data: { conversation?: MessagingConversationDetailDto } | null) => {
        if (!cancelled && data?.conversation && data.conversation.id === selectedIdRef.current) {
          setDetail(data.conversation);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [enabled, selectedId, detailTick]);

  const selected: MessagingConversationDto | null = useMemo(() => {
    if (!selectedId) return null;
    return (
      conversations.find((c) => c.id === selectedId) ??
      (detail && detail.id === selectedId ? detail : null)
    );
  }, [conversations, selectedId, detail]);

  const currentDetail = detail && detail.id === selectedId ? detail : null;

  const notifyError = useCallback((text: string) => toast(text), [toast]);

  const callPatch = useCallback(
    async (id: string, patch: MessagingConversationPatchInput) => {
      const res = await fetch(`/api/messaging/conversations/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(patch),
      });
      if (!res.ok) throw new Error("patch");
      const data = (await res.json()) as { conversation?: MessagingConversationDto };
      return data.conversation ?? null;
    },
    [],
  );

  const mergeConversation = useCallback(
    (next: MessagingConversationDto) => {
      setConversations((list) => list.map((x) => (x.id === next.id ? { ...x, ...next } : x)));
      setDetail((d) => (d && d.id === next.id ? { ...d, ...next } : d));
    },
    [setConversations],
  );

  // Mise à jour optimiste + rollback.
  const patchConversation = useCallback(
    async (c: MessagingConversationDto, patch: MessagingConversationPatchInput) => {
      const optimistic = (x: MessagingConversationDto): MessagingConversationDto => ({
        ...x,
        ...patch,
        ...(patch.markedUnread === false ? { unreadCount: 0, hasUnreadMention: false } : {}),
      });
      setConversations((list) => list.map((x) => (x.id === c.id ? optimistic(x) : x)));
      setDetail((d) => (d && d.id === c.id ? { ...d, ...optimistic(d) } : d));
      try {
        const updated = await callPatch(c.id, patch);
        if (updated) mergeConversation(updated);
        if (patch.markedUnread === false && c.unreadCount > 0) {
          void fetch(`/api/messaging/conversations/${c.id}/read`, { method: "POST" }).catch(
            () => undefined,
          );
        }
      } catch {
        setConversations((list) => list.map((x) => (x.id === c.id ? c : x)));
        setDetail((d) => (d && d.id === c.id ? { ...d, ...c } : d));
        notifyError("Action impossible pour le moment. Réessayez.");
      }
    },
    [callPatch, mergeConversation, notifyError, setConversations],
  );

  const removeFromList = useCallback(
    (id: string) => {
      setConversations((list) => list.filter((x) => x.id !== id));
      if (selectedIdRef.current === id) selectConv(null);
    },
    [selectConv, setConversations],
  );

  const deleteConversation = useCallback(
    async (c: MessagingConversationDto) => {
      removeFromList(c.id);
      try {
        const res = await fetch(`/api/messaging/conversations/${c.id}`, { method: "DELETE" });
        if (!res.ok) throw new Error("delete");
      } catch {
        notifyError("Suppression impossible. La conversation a été restaurée.");
        void refresh();
      }
    },
    [notifyError, refresh, removeFromList],
  );

  useMessagingStream({
    enabled,
    onEvent: (ev) => {
      if (ev.type.startsWith("call_")) {
        handleSseEvent(ev);
      }
      applyPresenceSse(ev);

      if (ev.type === "conversation_updated" && ev.conversationId === selectedIdRef.current) {
        setDetailTick((n) => n + 1);
      }

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
    try {
      const res = await fetch("/api/messaging/conversations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ peerUserId: peer.id }),
      });
      if (!res.ok) throw new Error("create");
      const data = (await res.json()) as { conversation?: MessagingConversationDto };
      await refresh();
      if (data.conversation) selectConv(data.conversation.id);
    } catch {
      notifyError("Impossible d’ouvrir la discussion.");
    }
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
      if (!res.ok) {
        notifyError("Création du groupe impossible.");
        return;
      }
      const data = (await res.json()) as { conversation?: MessagingConversationDto };
      await refresh();
      if (data.conversation) selectConv(data.conversation.id);
      setShowNewGroup(false);
      setGroupTitle("");
      setGroupMembers([]);
    } catch {
      notifyError("Création du groupe impossible.");
    } finally {
      setCreatingGroup(false);
    }
  };

  useEffect(() => {
    if (!call?.conversationId) return;
    const conv = conversations.find((c) => c.id === call.conversationId);
    if (conv && selectedIdRef.current !== conv.id) selectConv(conv.id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [call?.conversationId]);

  // Sélection d'une conversation marquée « non lue » : on retire le marquage.
  const onSelectFromList = (c: MessagingConversationDto) => {
    selectConv(c.id);
    if (c.markedUnread) void patchConversation(c, { markedUnread: false });
  };

  // Titre d'onglet : « (3) Messagerie — … ».
  const baseTitleRef = useRef<string | null>(null);
  useEffect(() => {
    if (baseTitleRef.current === null) {
      baseTitleRef.current = document.title.replace(/^\(\d+\+?\)\s*/, "");
    }
    const base = baseTitleRef.current;
    const n = conversations.filter(isUnread).length;
    document.title = n > 0 ? `(${n > 99 ? "99+" : n}) ${base}` : base;
  }, [conversations]);
  useEffect(
    () => () => {
      if (baseTitleRef.current !== null) document.title = baseTitleRef.current;
    },
    [],
  );

  // Raccourcis : Ctrl+K ou « / » focalisent la recherche.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement | null;
      const typing =
        el &&
        (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setMobileOpen(false);
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
      } else if (e.key === "/" && !typing && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        setMobileOpen(false);
        searchInputRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const drawerCall = async (fn: () => Promise<Response>, failure: string) => {
    try {
      const res = await fn();
      if (!res.ok) throw new Error("x");
      return (await res.json().catch(() => ({}))) as {
        conversation?: MessagingConversationDetailDto;
      };
    } catch {
      notifyError(failure);
      return null;
    }
  };

  const applyDetail = (next: MessagingConversationDetailDto | undefined) => {
    if (!next) {
      setDetailTick((n) => n + 1);
      return;
    }
    setDetail((d) => (d && d.id === next.id ? { ...d, ...next } : next));
    mergeConversation(next);
  };

  const inSplitCall = Boolean(call && call.phase !== "idle" && call.viewMode === "split");
  const unreadConversations = conversations.filter(isUnread).length;

  return (
    <DashboardThemeRoot>
      <MessagingCallOverlay embedSplitInPage />
      <div className="mx-auto flex h-[calc(100dvh-5.5rem)] w-full max-w-[90rem] flex-col gap-3 p-3 sm:p-4 lg:p-5">
        <header
          className={`flex-wrap items-center justify-between gap-3 px-1 ${
            mobileOpen && section === "messages" ? "hidden md:flex" : "flex"
          }`}
        >
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
                  : unreadConversations > 0 || totalUnread > 0
                    ? `${unreadConversations} conversation${unreadConversations > 1 ? "s" : ""} non lue${unreadConversations > 1 ? "s" : ""}`
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
            className={`grid min-h-0 flex-1 grid-cols-1 gap-3 transition-opacity duration-200 ${
              inSplitCall
                ? "lg:grid-cols-[minmax(260px,17rem)_minmax(0,1.15fr)_minmax(280px,1fr)]"
                : "md:grid-cols-[minmax(280px,20rem)_minmax(0,1fr)]"
            }`}
          >
            <aside
              className={`min-h-0 flex-col overflow-hidden rounded-[1.75rem] border border-black/6 bg-[#eceeea] shadow-[0_1px_0_rgba(0,0,0,0.03)] ${
                mobileOpen ? "hidden md:flex" : "flex"
              }`}
            >
              <ConversationList
                conversations={conversations}
                meId={currentUserId}
                selectedId={selectedId}
                users={users}
                statusOf={statusOf}
                statusForConversation={statusForConversation}
                searchInputRef={searchInputRef}
                onSelect={onSelectFromList}
                onOpenConversationId={(id) => selectConv(id)}
                onStartWithPeer={(p) => void startWithPeer(p)}
                onPatch={(c, patch) => void patchConversation(c, patch)}
                onDelete={(c) => void deleteConversation(c)}
                notify={notifyError}
              />
            </aside>

            {inSplitCall ? (
              <section className="hidden min-h-0 overflow-hidden rounded-[1.75rem] border border-black/6 lg:block">
                <MessagingCallStage className="h-full" />
              </section>
            ) : null}

            <section className={`min-h-0 flex-col ${mobileOpen ? "flex" : "hidden md:flex"}`}>
              {selected ? (
                <>
                  <button
                    type="button"
                    onClick={() => selectConv(null)}
                    className="mb-2 inline-flex items-center gap-1.5 self-start rounded-2xl bg-[#eceeea] px-3 py-1.5 text-xs font-semibold text-[var(--dash-ink)] md:hidden"
                  >
                    <span aria-hidden>←</span> Conversations
                  </button>
                  <div className="min-h-0 flex-1">
                    <MessagingConversationPanel
                      key={selected.id}
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
                      members={currentDetail?.members}
                      pinnedMessages={currentDetail?.pinnedMessages}
                      onOpenDetails={() => setShowDetails(true)}
                      onForwardRequest={setForwardMessage}
                      onStartVideoCall={() =>
                        void startCall({
                          conversationId: selected.id,
                          title: selected.title,
                          kind: selected.kind,
                        })
                      }
                    />
                  </div>
                </>
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

      {showDetails && currentDetail ? (
        <ConversationDetailsDrawer
          detail={currentDetail}
          meId={currentUserId}
          users={users}
          statusOf={statusOf}
          statusForConversation={statusForConversation}
          onClose={() => setShowDetails(false)}
          onToggleMute={() => void patchConversation(currentDetail, { muted: !currentDetail.muted })}
          onRename={async (title) => {
            try {
              const updated = await callPatch(currentDetail.id, { title });
              if (updated) mergeConversation(updated);
              else setDetailTick((n) => n + 1);
              return true;
            } catch {
              notifyError("Renommage impossible.");
              return false;
            }
          }}
          onAddMembers={async (userIds) => {
            const data = await drawerCall(
              () =>
                fetch(`/api/messaging/conversations/${currentDetail.id}/members`, {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({ userIds }),
                }),
              "Ajout impossible.",
            );
            if (!data) return false;
            applyDetail(data.conversation);
            return true;
          }}
          onRemoveMember={async (userId) => {
            const data = await drawerCall(
              () =>
                fetch(
                  `/api/messaging/conversations/${currentDetail.id}/members/${encodeURIComponent(userId)}`,
                  { method: "DELETE" },
                ),
              "Retrait impossible.",
            );
            if (!data) return false;
            applyDetail(data.conversation);
            return true;
          }}
          onLeave={async () => {
            const data = await drawerCall(
              () =>
                fetch(
                  `/api/messaging/conversations/${currentDetail.id}/members/${encodeURIComponent(currentUserId)}`,
                  { method: "DELETE" },
                ),
              "Impossible de quitter le groupe.",
            );
            if (!data) return false;
            setShowDetails(false);
            removeFromList(currentDetail.id);
            return true;
          }}
          onStartVideoCall={() =>
            void startCall({
              conversationId: currentDetail.id,
              title: currentDetail.title,
              kind: currentDetail.kind,
            })
          }
        />
      ) : null}

      {toastNode}

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
                    selectConv(c.id);
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
