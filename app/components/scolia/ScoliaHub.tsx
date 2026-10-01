"use client";

import { useCallback, useEffect, useState } from "react";
import ChatbotBubble from "@/app/components/ChatbotBubble";
import ScoliaAiMark from "@/app/components/ScoliaAiMark";
import { askScolia, openScolia } from "@/app/lib/brain-ai/scolia-ask";
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

const SUGGESTIONS = [
  "Ouvre les documents de préinscription d’un élève",
  "Qu’est-ce qui se passe aujourd’hui ?",
  "Réserve une salle demain matin",
  "Montre les sorties scolaires en cours",
  "Passe un élève en interne",
];

/**
 * Hub chat-first pour /dashboard : historique + suggestions + ScolIA plein écran.
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
    // Soft reload of pageMode bubble state via full navigation to /scolia-ai alternative:
    // On dashboard we remount by key — force local welcome via ask with empty then clear.
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
      // Inject into localStorage memory so ChatbotBubble picks it up
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
    <div className="flex min-h-[calc(100dvh-1.5rem)] w-full flex-col gap-3 lg:flex-row lg:gap-4">
      {/* Historique */}
      <aside
        className={`shrink-0 overflow-hidden rounded-[1.5rem] border border-white/70 bg-white/70 shadow-sm backdrop-blur-xl transition-all lg:w-64 ${
          historyOpen ? "max-h-[40vh] lg:max-h-none" : "max-h-12"
        }`}
      >
        <div className="flex items-center justify-between gap-2 border-b border-slate-100 px-3 py-2.5">
          <button
            type="button"
            className="flex items-center gap-2 text-left"
            onClick={() => setHistoryOpen((v) => !v)}
          >
            <ScoliaAiMark className="h-6 w-6" />
            <span className="text-xs font-semibold uppercase tracking-wide text-slate-600">
              Historique
            </span>
          </button>
          <button
            type="button"
            onClick={startNew}
            className="rounded-full bg-slate-900 px-2.5 py-1 text-[11px] font-semibold text-white hover:bg-black"
          >
            Nouveau
          </button>
        </div>
        {historyOpen ? (
          <ul className="max-h-[32vh] space-y-0.5 overflow-y-auto p-2 lg:max-h-[calc(100dvh-8rem)]">
            {loadingList ? (
              <li className="px-2 py-3 text-xs text-slate-400">Chargement…</li>
            ) : conversations.length === 0 ? (
              <li className="px-2 py-3 text-xs text-slate-400">
                Aucune conversation enregistrée pour l’instant.
              </li>
            ) : (
              conversations.map((c) => (
                <li key={c.id}>
                  <button
                    type="button"
                    onClick={() => void resumeConversation(c.id)}
                    className="w-full rounded-xl px-2.5 py-2 text-left text-sm text-slate-700 transition hover:bg-slate-100"
                  >
                    <span className="line-clamp-2 font-medium">{c.title}</span>
                    <span className="mt-0.5 block text-[10px] text-slate-400">
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

      {/* Chat principal */}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-[1.75rem] border border-white/70 bg-white/75 shadow-sm backdrop-blur-xl">
        <div className="border-b border-slate-100 px-4 py-3 sm:px-5">
          <div className="flex flex-wrap items-center gap-2">
            <ScoliaAiMark className="h-8 w-8" />
            <div>
              <h1 className="text-lg font-semibold text-slate-900">{SCOLIA_AI_NAME}</h1>
              <p className="text-xs text-slate-500">
                Parlez ou écrivez — j’ouvre les pages et j’agis avec votre validation.
              </p>
            </div>
          </div>
          <div className="mt-3 flex flex-wrap gap-1.5">
            {SUGGESTIONS.map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => askScolia(s)}
                className="rounded-full border border-slate-200 bg-white px-3 py-1.5 text-left text-[11px] font-medium text-slate-700 transition hover:border-slate-300 hover:bg-slate-50"
              >
                {s}
              </button>
            ))}
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-hidden">
          <ChatbotBubble pageMode embedded />
        </div>
      </div>
    </div>
  );
}
