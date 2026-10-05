"use client";

import { useCallback, useEffect, useState } from "react";
import ChatbotBubble from "@/app/components/ChatbotBubble";
import ScoliaAiMark from "@/app/components/ScoliaAiMark";
import { openScolia } from "@/app/lib/brain-ai/scolia-ask";
import {
  clearScoliaMemory,
  defaultWelcomeMessage,
  SCOLIA_AI_NAME,
} from "@/app/lib/brain-ai/scolia-memory";

type ConversationSummary = {
  id: string;
  title: string;
  updatedAt: string;
  lastMessageAt: string | null;
};

/**
 * Hub chat-first pour /dashboard — bento minimaliste, sans raccourcis.
 */
export default function ScoliaHub() {
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [historyOpen, setHistoryOpen] = useState(true);
  const [loadingList, setLoadingList] = useState(true);

  const refreshList = useCallback(async () => {
    try {
      const res = await fetch("/api/chatbot/conversations");
      if (!res.ok) return;
      const data = (await res.json()) as { conversations?: ConversationSummary[] };
      setConversations(Array.isArray(data.conversations) ? data.conversations : []);
    } catch {
      /* ignore */
    } finally {
      setLoadingList(false);
    }
  }, []);

  useEffect(() => {
    void refreshList();
    const t = window.setInterval(() => void refreshList(), 45_000);
    return () => window.clearInterval(t);
  }, [refreshList]);

  const startNew = () => {
    clearScoliaMemory();
    openScolia();
    window.location.href = "/dashboard?new=1";
  };

  const resumeConversation = async (id: string) => {
    try {
      const res = await fetch(`/api/chatbot/conversations/${encodeURIComponent(id)}`);
      if (!res.ok) return;
      const data = (await res.json()) as {
        conversation?: {
          id: string;
          state: Record<string, unknown>;
          messages: Array<{ role: "user" | "assistant"; content: string }>;
        };
      };
      const conv = data.conversation;
      if (!conv) return;
      const messages =
        conv.messages.length > 0
          ? conv.messages.map((m) => ({ role: m.role, content: m.content }))
          : [defaultWelcomeMessage()];
      const { saveScoliaMemory } = await import("@/app/lib/brain-ai/scolia-memory");
      saveScoliaMemory({
        messages,
        conversationState: { ...conv.state, conversationId: conv.id },
        pendingConfirmation: null,
        pendingChoices: null,
      });
      window.location.href = "/dashboard?resume=1";
    } catch {
      /* ignore */
    }
  };

  return (
    <div className="grid min-h-[calc(100dvh-1.5rem)] w-full grid-cols-1 gap-3 lg:grid-cols-[15.5rem_minmax(0,1fr)] lg:gap-3.5">
      {/* Bento — historique */}
      <aside
        className={`flex flex-col overflow-hidden rounded-[1.75rem] border border-black/6 bg-[#eceeea] shadow-[0_1px_0_rgba(0,0,0,0.03)] transition-all ${
          historyOpen ? "max-h-[38vh] lg:max-h-none" : "max-h-14"
        }`}
      >
        <div className="flex items-center justify-between gap-2 px-3.5 py-3">
          <button
            type="button"
            className="flex min-w-0 items-center gap-2.5 text-left"
            onClick={() => setHistoryOpen((v) => !v)}
            aria-expanded={historyOpen}
          >
            <span className="flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-[var(--dash-ink)]">
              <ScoliaAiMark size="sm" inverted fill />
            </span>
            <span className="truncate text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--dash-mid)]">
              Historique
            </span>
          </button>
          <button
            type="button"
            onClick={startNew}
            className="shrink-0 rounded-2xl bg-[var(--dash-ink)] px-3 py-1.5 text-[11px] font-semibold text-white transition hover:opacity-90"
          >
            Nouveau
          </button>
        </div>
        {historyOpen ? (
          <ul className="min-h-0 flex-1 space-y-1 overflow-y-auto px-2.5 pb-3 lg:max-h-[calc(100dvh-9rem)]">
            {loadingList ? (
              <li className="px-3 py-4 text-xs text-neutral-500">Chargement…</li>
            ) : conversations.length === 0 ? (
              <li className="rounded-2xl px-3 py-4 text-xs leading-relaxed text-neutral-500">
                Vos conversations apparaîtront ici.
              </li>
            ) : (
              conversations.map((c) => (
                <li key={c.id}>
                  <button
                    type="button"
                    onClick={() => void resumeConversation(c.id)}
                    className="w-full rounded-2xl px-3 py-2.5 text-left transition hover:bg-white/70"
                  >
                    <span className="line-clamp-2 text-sm font-semibold text-[var(--dash-ink)]">
                      {c.title}
                    </span>
                    <span className="mt-1 block text-[10px] font-medium text-neutral-500">
                      {new Date(c.updatedAt).toLocaleString("fr-FR", {
                        day: "2-digit",
                        month: "short",
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </span>
                  </button>
                </li>
              ))
            )}
          </ul>
        ) : null}
      </aside>

      {/* Bento — chat hero */}
      <section className="flex min-h-[70dvh] min-w-0 flex-col overflow-hidden rounded-[1.75rem] border border-black/6 bg-white/80 shadow-[0_1px_0_rgba(0,0,0,0.03)] backdrop-blur-xl lg:min-h-0">
        <header className="flex items-center gap-3 border-b border-black/5 px-5 py-4 sm:px-6">
          <span className="flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-[var(--dash-ink)] shadow-sm">
            <ScoliaAiMark size="sm" inverted fill />
          </span>
          <div className="min-w-0">
            <h1 className="truncate text-lg font-semibold tracking-tight text-[var(--dash-ink)]">
              {SCOLIA_AI_NAME}
            </h1>
            <p className="truncate text-xs text-neutral-500">
              Parlez ou écrivez — j’ouvre et j’agis avec votre validation.
            </p>
          </div>
          <span
            className="ml-auto hidden h-2.5 w-2.5 shrink-0 rounded-full bg-[var(--dash-lime)] ring-4 ring-[color:var(--dash-lime)]/25 sm:block"
            title="En ligne"
            aria-hidden
          />
        </header>
        <div className="min-h-0 flex-1 overflow-hidden">
          <ChatbotBubble pageMode embedded />
        </div>
      </section>
    </div>
  );
}
