"use client";

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type DragEvent,
  type FormEvent,
} from "react";
import type {
  TeamsChatMessage,
  TeamsChatPerson,
  TeamsChatStatus,
  TeamsChatSummary,
} from "@/app/lib/teams-chat/types";
import {
  acquireTeamsChatToken,
  getTeamsChatAdminConsentUrl,
  isTeamsChatAdminConsentError,
  type TeamsChatMsalSession,
} from "@/app/lib/teams-chat/msal-client";

const HEADS_COLLAPSED_KEY = "scola.teamsChat.headsCollapsed";
const DOCKED_HEADS_KEY = "scola.teamsChat.dockedHeadIds";

type Props = { initialStatus: TeamsChatStatus };

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean).slice(0, 2);
  const s = parts.map((p) => p[0]?.toUpperCase() || "").join("");
  return s || "?";
}

function PersonAvatar({
  person,
  token,
  size = "md",
}: {
  person: Pick<TeamsChatPerson, "id" | "displayName">;
  token: string | null;
  size?: "sm" | "md";
}) {
  const [src, setSrc] = useState<string | null>(null);
  const dim = size === "sm" ? "h-8 w-8 text-[11px]" : "h-12 w-12 text-sm";

  useEffect(() => {
    if (!token || !person.id) return;
    let alive = true;
    let objectUrl: string | null = null;
    fetch(`/api/teams-chat/people/${encodeURIComponent(person.id)}/photo`, {
      headers: { "X-Graph-Access-Token": token },
    })
      .then(async (r) => {
        if (!r.ok) return;
        const blob = await r.blob();
        objectUrl = URL.createObjectURL(blob);
        if (alive) setSrc(objectUrl);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [person.id, token]);

  return (
    <span
      className={`relative flex shrink-0 items-center justify-center overflow-hidden rounded-full border border-white/40 bg-indigo-800 text-white ${dim}`}
    >
      {src ? (
        <img src={src} alt="" className="h-full w-full object-cover" />
      ) : (
        initials(person.displayName)
      )}
    </span>
  );
}

export default function TeamsChatOverlay({ initialStatus }: Props) {
  const [status, setStatus] = useState(initialStatus);
  const [chats, setChats] = useState<TeamsChatSummary[]>([]);
  const [open, setOpen] = useState(false);
  const [headsCollapsed, setHeadsCollapsed] = useState(false);
  const [dockedIds, setDockedIds] = useState<string[]>([]);
  const [activeChat, setActiveChat] = useState<TeamsChatSummary | null>(null);
  const [messages, setMessages] = useState<TeamsChatMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [query, setQuery] = useState("");
  const [people, setPeople] = useState<TeamsChatPerson[]>([]);
  const [loadingChats, setLoadingChats] = useState(false);
  const [loadingMessages, setLoadingMessages] = useState(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dragOverDock, setDragOverDock] = useState(false);
  const [graphToken, setGraphToken] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [adminConsentUrl, setAdminConsentUrl] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const linked = Boolean(graphToken);

  const graphHeaders = useCallback(
    (extra?: HeadersInit): HeadersInit => ({
      ...(extra ?? {}),
      ...(graphToken ? { "X-Graph-Access-Token": graphToken } : {}),
    }),
    [graphToken],
  );

  useEffect(() => {
    let cancelled = false;
    acquireTeamsChatToken(false)
      .then((session: TeamsChatMsalSession | null) => {
        if (cancelled || !session) return;
        setGraphToken(session.accessToken);
        setStatus((s) => ({
          ...s,
          linked: true,
          me: { displayName: session.accountLabel },
        }));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    try {
      setHeadsCollapsed(localStorage.getItem(HEADS_COLLAPSED_KEY) === "1");
      const raw = localStorage.getItem(DOCKED_HEADS_KEY);
      if (raw) setDockedIds(JSON.parse(raw) as string[]);
    } catch {
      /* ignore */
    }
  }, []);

  const persistDocked = (ids: string[]) => {
    setDockedIds(ids);
    try {
      localStorage.setItem(DOCKED_HEADS_KEY, JSON.stringify(ids));
    } catch {
      /* ignore */
    }
  };

  const toggleHeads = () => {
    const next = !headsCollapsed;
    setHeadsCollapsed(next);
    try {
      localStorage.setItem(HEADS_COLLAPSED_KEY, next ? "1" : "0");
    } catch {
      /* ignore */
    }
  };

  const loadChats = useCallback(async () => {
    if (!graphToken) return;
    setLoadingChats(true);
    try {
      const res = await fetch("/api/teams-chat/chats", { headers: graphHeaders() });
      const data = (await res.json()) as { chats?: TeamsChatSummary[]; error?: string; code?: string };
      if (data.code === "TEAMS_UNLINKED") {
        setGraphToken(null);
        setStatus((s) => ({ ...s, linked: false }));
        return;
      }
      if (!res.ok) throw new Error(data.error || "Chargement impossible.");
      setChats(data.chats ?? []);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement impossible.");
    } finally {
      setLoadingChats(false);
    }
  }, [graphToken, graphHeaders]);

  const loadMessages = useCallback(async (chatId: string, quiet = false) => {
    if (!graphToken) return;
    if (!quiet) setLoadingMessages(true);
    try {
      const res = await fetch(`/api/teams-chat/chats/${encodeURIComponent(chatId)}/messages`, {
        headers: graphHeaders(),
      });
      const data = (await res.json()) as { messages?: TeamsChatMessage[]; error?: string };
      if (!res.ok) throw new Error(data.error || "Messages indisponibles.");
      setMessages(data.messages ?? []);
    } catch (e) {
      if (!quiet) setError(e instanceof Error ? e.message : "Messages indisponibles.");
    } finally {
      if (!quiet) setLoadingMessages(false);
    }
  }, [graphToken, graphHeaders]);

  useEffect(() => {
    void loadChats();
  }, [loadChats]);

  useEffect(() => {
    if (!linked) return;
    const t = window.setInterval(() => void loadChats(), 25000);
    return () => window.clearInterval(t);
  }, [linked, loadChats]);

  useEffect(() => {
    if (!open || !activeChat) return;
    void loadMessages(activeChat.id);
    const t = window.setInterval(() => void loadMessages(activeChat.id, true), 8000);
    return () => window.clearInterval(t);
  }, [open, activeChat, loadMessages]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, open, activeChat]);

  useEffect(() => {
    if (!open || !linked) return;
    const q = query.trim();
    if (q.length < 2) {
      setPeople([]);
      return;
    }
    const t = window.setTimeout(() => {
      fetch(`/api/teams-chat/people?q=${encodeURIComponent(q)}`, { headers: graphHeaders() })
        .then((r) => r.json() as Promise<{ people?: TeamsChatPerson[] }>)
        .then((data) => setPeople(data.people ?? []))
        .catch(() => setPeople([]));
    }, 280);
    return () => window.clearTimeout(t);
  }, [query, open, linked, graphHeaders]);

  const visibleHeads = useMemo(() => {
    const docked = new Set(dockedIds);
    return chats.filter((c) => !docked.has(c.id)).slice(0, 3);
  }, [chats, dockedIds]);

  const openChat = (chat: TeamsChatSummary) => {
    setActiveChat(chat);
    setOpen(true);
    setQuery("");
    setPeople([]);
    setMessages([]);
  };

  const startChatWith = async (person: TeamsChatPerson) => {
    setError(null);
    try {
      const res = await fetch("/api/teams-chat/chats", {
        method: "POST",
        headers: graphHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ otherUserId: person.id }),
      });
      const data = (await res.json()) as { chatId?: string; error?: string };
      if (!res.ok || !data.chatId) throw new Error(data.error || "Ouverture impossible.");
      const existing = chats.find((c) => c.id === data.chatId);
      const chat: TeamsChatSummary = existing ?? {
        id: data.chatId,
        other: person,
      };
      persistDocked(dockedIds.filter((id) => id !== chat.id));
      openChat(chat);
      void loadChats();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Ouverture impossible.");
    }
  };

  const send = async (e: FormEvent) => {
    e.preventDefault();
    if (!activeChat || !draft.trim() || sending) return;
    setSending(true);
    setError(null);
    try {
      const res = await fetch(
        `/api/teams-chat/chats/${encodeURIComponent(activeChat.id)}/messages`,
        {
          method: "POST",
          headers: graphHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({ text: draft }),
        },
      );
      const data = (await res.json()) as { messages?: TeamsChatMessage[]; error?: string };
      if (!res.ok) throw new Error(data.error || "Envoi impossible.");
      setDraft("");
      setMessages(data.messages ?? []);
      void loadChats();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Envoi impossible.");
    } finally {
      setSending(false);
    }
  };

  const onDropOnDock = (e: DragEvent) => {
    e.preventDefault();
    setDragOverDock(false);
    const chatId = e.dataTransfer.getData("text/teams-chat-id");
    if (!chatId) return;
    persistDocked([...new Set([...dockedIds, chatId])]);
    if (activeChat?.id === chatId) setOpen(false);
  };

  const connectMicrosoft = async () => {
    setConnecting(true);
    setError(null);
    try {
      const session = await acquireTeamsChatToken(true);
      if (!session) throw new Error("Connexion Microsoft annulée.");
      setGraphToken(session.accessToken);
      setStatus((s) => ({
        ...s,
        linked: true,
        me: { displayName: session.accountLabel },
      }));
    } catch (e) {
      const msg = isTeamsChatAdminConsentError(e)
        ? "Approbation administrateur requise : un admin Entra (DSI) doit autoriser DocsLaPro pour Chat."
        : e instanceof Error
          ? e.message
          : "Connexion Microsoft impossible.";
      setError(msg);
      try {
        setAdminConsentUrl(await getTeamsChatAdminConsentUrl());
      } catch {
        /* ignore */
      }
    } finally {
      setConnecting(false);
    }
  };

  return (
    <>
      {open ? (
        <div
          className="fixed inset-0 z-[128] bg-slate-900/20 lg:hidden"
          onClick={() => setOpen(false)}
          aria-hidden
        />
      ) : null}

      {open ? (
      <div
        className="fixed inset-0 z-[128] flex h-[100dvh] flex-col overflow-hidden border-0 bg-white/90 backdrop-blur-2xl lg:inset-auto lg:bottom-20 lg:right-[5.5rem] lg:h-[580px] lg:w-[min(92vw,400px)] lg:rounded-[1.5rem] lg:border lg:border-white/55 lg:bg-white/70 lg:shadow-[0_24px_60px_-28px_rgba(15,23,42,0.4)]"
      >
        <div className="flex items-center gap-2 border-b border-slate-200/70 px-3 py-2.5">
          {activeChat ? (
            <button
              type="button"
              className="rounded-full px-2 py-1 text-sm text-slate-600 hover:bg-slate-100"
              onClick={() => setActiveChat(null)}
            >
              ←
            </button>
          ) : null}
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold text-slate-900">
              {activeChat ? activeChat.other.displayName : "Messages"}
            </p>
            <p className="truncate text-[11px] text-slate-500">
              {linked
                ? status.me?.displayName || "Compte Microsoft lié"
                : "Liez votre compte Microsoft (Teams)"}
            </p>
          </div>
          <button
            type="button"
            className="rounded-full px-2 py-1 text-slate-500 hover:bg-slate-100"
            onClick={() => setOpen(false)}
            aria-label="Fermer"
          >
            ×
          </button>
        </div>

        {!linked ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
            <p className="text-sm text-slate-600">
              La messagerie interne utilise Teams. Chaque personne lie son propre compte A1 ou A3.
            </p>
            {error || status.error ? (
              <p className="text-xs text-rose-600">{error || status.error}</p>
            ) : null}
            {adminConsentUrl ? (
              <p className="text-left text-[11px] leading-relaxed text-slate-600">
                Envoyez ce lien à un administrateur Microsoft 365 (il se connecte et clique sur
                Accepter) :
                <a
                  href={adminConsentUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="mt-1 block break-all text-indigo-700 underline"
                >
                  {adminConsentUrl}
                </a>
              </p>
            ) : null}
            <button
              type="button"
              onClick={() => void connectMicrosoft()}
              disabled={connecting}
              className="rounded-full bg-indigo-700 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-800 disabled:opacity-50"
            >
              {connecting ? "Connexion…" : "Lier mon compte Microsoft"}
            </button>
          </div>
        ) : activeChat ? (
          <>
            <div className="flex-1 space-y-2 overflow-y-auto px-3 py-3">
              {loadingMessages && messages.length === 0 ? (
                <p className="text-center text-xs text-slate-400">Chargement…</p>
              ) : null}
              {messages.map((m) => (
                <div key={m.id} className={`flex ${m.fromMe ? "justify-end" : "justify-start"}`}>
                  <div
                    className={`max-w-[80%] rounded-2xl px-3 py-2 text-sm leading-snug ${
                      m.fromMe
                        ? "bg-indigo-700 text-white"
                        : "bg-slate-100 text-slate-800"
                    }`}
                  >
                    {m.text}
                  </div>
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>
            <form onSubmit={send} className="flex gap-2 border-t border-slate-200/70 p-3">
              <input
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                placeholder="Écrire un message…"
                className="min-w-0 flex-1 rounded-full border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:border-indigo-400"
              />
              <button
                type="submit"
                disabled={sending || !draft.trim()}
                className="rounded-full bg-indigo-700 px-3 py-2 text-sm font-medium text-white disabled:opacity-40"
              >
                Envoyer
              </button>
            </form>
          </>
        ) : (
          <>
            <div className="px-3 pt-3">
              <input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Rechercher un collègue…"
                className="w-full rounded-full border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:border-indigo-400"
              />
            </div>
            <div className="flex-1 overflow-y-auto px-2 py-2">
              {error ? <p className="px-2 text-xs text-rose-600">{error}</p> : null}
              {query.trim().length >= 2
                ? people.map((p) => (
                    <button
                      key={p.id}
                      type="button"
                      onClick={() => void startChatWith(p)}
                      className="flex w-full items-center gap-2 rounded-xl px-2 py-2 text-left hover:bg-slate-100"
                    >
                      <PersonAvatar person={p} token={graphToken} size="sm" />
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-medium text-slate-800">
                          {p.displayName}
                        </span>
                        <span className="block truncate text-[11px] text-slate-500">
                          {p.jobTitle || p.mail || p.userPrincipalName}
                        </span>
                      </span>
                    </button>
                  ))
                : chats.map((c) => (
                    <button
                      key={c.id}
                      type="button"
                      onClick={() => openChat(c)}
                      className="flex w-full items-center gap-2 rounded-xl px-2 py-2 text-left hover:bg-slate-100"
                    >
                      <PersonAvatar person={c.other} token={graphToken} size="sm" />
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-sm font-medium text-slate-800">
                          {c.other.displayName}
                        </span>
                        <span className="block truncate text-[11px] text-slate-500">
                          {c.lastPreview || "Conversation Teams"}
                        </span>
                      </span>
                    </button>
                  ))}
              {loadingChats && chats.length === 0 && query.trim().length < 2 ? (
                <p className="px-2 text-xs text-slate-400">Chargement des conversations…</p>
              ) : null}
            </div>
          </>
        )}
      </div>
      ) : null}

      <div className="fixed bottom-4 right-[5.25rem] z-[129] flex flex-col-reverse items-center gap-2">
        <button
          type="button"
          onClick={() => {
            setOpen((v) => !v);
            if (!open) setActiveChat(null);
          }}
          onDragOver={(e) => {
            e.preventDefault();
            setDragOverDock(true);
          }}
          onDragLeave={() => setDragOverDock(false)}
          onDrop={onDropOnDock}
          className={`flex h-14 w-14 cursor-pointer items-center justify-center rounded-full border shadow-[0_14px_34px_rgba(49,46,129,0.35)] backdrop-blur-xl transition-all hover:scale-[1.04] active:scale-[0.97] ${
            dragOverDock
              ? "border-indigo-200 bg-indigo-600"
              : "border-indigo-300/40 bg-indigo-950/80 hover:border-indigo-200/50"
          }`}
          aria-label={open ? "Fermer Messages" : "Ouvrir Messages"}
        >
          <span className="text-[11px] font-semibold leading-tight text-white">Msg</span>
        </button>

        {!headsCollapsed
          ? visibleHeads.map((c) => (
              <button
                key={c.id}
                type="button"
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData("text/teams-chat-id", c.id);
                  e.dataTransfer.effectAllowed = "move";
                }}
                onClick={() => openChat(c)}
                title={`${c.other.displayName} — glisser sur Messages pour ranger`}
                className="rounded-full shadow-[0_8px_20px_rgba(15,23,42,0.25)] transition hover:scale-105"
              >
                <PersonAvatar person={c.other} token={graphToken} />
              </button>
            ))
          : null}

        {visibleHeads.length > 0 ? (
          <button
            type="button"
            onClick={toggleHeads}
            className="flex h-7 w-7 items-center justify-center rounded-full border border-slate-200 bg-white/90 text-xs text-slate-600 shadow-sm hover:bg-white"
            aria-label={headsCollapsed ? "Afficher les conversations" : "Replier les conversations"}
            title={headsCollapsed ? "Afficher" : "Replier"}
          >
            {headsCollapsed ? "▴" : "▾"}
          </button>
        ) : null}
      </div>
    </>
  );
}
