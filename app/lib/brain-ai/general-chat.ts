/**
 * Mode « hors métier » de ScolIA : un LLM classique, sans outils ni cadre ENT.
 * Réglages par défaut du modèle (pas de température imposée). Seulement trois garde-fous :
 * ne jamais prétendre avoir agi dans l'ENT, ne rien inventer sur de vraies personnes,
 * français par défaut. Les noms connus sont masqués avant l'envoi puis remis.
 */
import type { BrainConversationState, BrainChatResponse } from "@/app/lib/brain-ai/types";
import {
  MISTRAL_LARGE_MODEL,
  MISTRAL_SMALL_MODEL,
  mistralMessageText,
} from "@/app/lib/mistral-models";

const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";

type Turn = { role: "user" | "assistant"; content: string };

export function generalSystemPrompt(firstName?: string): string {
  return (
    `Tu es ScolIA, l'assistant IA de l'établissement scolaire, utilisé par le personnel.` +
    (firstName ? ` Tu parles avec ${firstName}.` : "") +
    `\nRéponds en français, sauf si on te demande une autre langue.\n` +
    `Dans cette conversation tu n'as pas accès à la plateforme : ne prétends jamais avoir envoyé, ` +
    `enregistré ou consulté quoi que ce soit dans l'ENT. Si l'utilisateur veut une action (envoyer ` +
    `un message, consulter un élève…), dis-lui de te la demander directement, par exemple « envoie-le à … ».\n` +
    `N'invente aucun fait sur de vraies personnes (élèves, familles, collègues). Les étiquettes entre ` +
    `crochets comme [Élève 1] ou [Parent 2 nom] remplacent des noms réels : recopie-les telles quelles.`
  );
}

/** Historique réel (rôles conservés), sans messages vides, borné. */
export function toTurns(history: Array<{ role: string; content: string }>): Turn[] {
  return history
    .filter((m) => (m.role === "user" || m.role === "assistant") && typeof m.content === "string")
    .map((m) => ({ role: m.role as Turn["role"], content: m.content.trim() }))
    .filter((m) => m.content)
    .slice(-12);
}

async function callMistral(model: string, messages: unknown[], apiKey: string): Promise<Response | null> {
  try {
    return await fetch(MISTRAL_URL, {
      method: "POST",
      signal: AbortSignal.timeout(90_000),
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${apiKey}` },
      body: JSON.stringify({ model, messages }),
    });
  } catch {
    return null;
  }
}

export async function runGeneralChat(input: {
  message: string;
  history: Array<{ role: string; content: string }>;
  apiKey: string;
  firstName?: string;
  etablissementId?: string | null;
  conversationState: BrainConversationState;
}): Promise<BrainChatResponse> {
  const turns = toTurns(input.history);
  const texts = [...turns.map((t) => t.content), input.message];

  let masked = texts;
  let restore = (s: string) => s;
  if (input.etablissementId) {
    const { pseudonymizeForLlm } = await import("@/app/lib/brain-ai/pseudonymize");
    const p = await pseudonymizeForLlm(input.etablissementId, texts);
    masked = p.texts;
    restore = p.restore;
  }

  const messages = [
    { role: "system", content: generalSystemPrompt(input.firstName) },
    ...turns.map((t, i) => ({ role: t.role, content: masked[i] })),
    { role: "user", content: masked[masked.length - 1] },
  ];

  let res = await callMistral(MISTRAL_LARGE_MODEL, messages, input.apiKey);
  if (!res || (!res.ok && [429, 500, 502, 503, 504].includes(res.status))) {
    console.warn("[brain-ai] mode général : Large 4 indisponible, secours Small 4", { status: res?.status });
    res = await callMistral(MISTRAL_SMALL_MODEL, messages, input.apiKey);
  }
  const base = {
    conversationState: input.conversationState,
    pendingConfirmation: null,
    pendingChoices: null,
  };
  if (!res || !res.ok) {
    return { ...base, answer: "Le service IA est temporairement indisponible. Réessaie dans quelques secondes." };
  }
  const data = await res.json().catch(() => null);
  const text = mistralMessageText(data?.choices?.[0]?.message?.content).trim();
  return { ...base, answer: restore(text) || "Je n'ai pas pu formuler de réponse pour le moment." };
}
