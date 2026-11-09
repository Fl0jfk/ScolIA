import "server-only";

import { and, desc, eq, gt, isNull, ne } from "drizzle-orm";
import { getDb } from "@/db/index";
import { messagingMessage, messagingParticipant, user } from "@/db/schema";
import {
  MISTRAL_LARGE_MODEL,
  MISTRAL_SMALL_MODEL,
  mistralMessageText,
  mistralModelOptions,
} from "@/app/lib/mistral-models";
import {
  buildAssistPrompt,
  type AssistMessage,
  type AssistMode,
  type AssistTone,
} from "@/app/lib/messaging/assist-prompt";

export { buildAssistPrompt } from "@/app/lib/messaging/assist-prompt";
export type { AssistMode, AssistTone } from "@/app/lib/messaging/assist-prompt";

const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";
const CALL_TIMEOUT_MS = 30_000;
const THREAD_LIMIT = 40;

export class AssistError extends Error {
  readonly status: number;
  readonly code: string;
  constructor(message: string, status: number, code: string) {
    super(message);
    this.name = "AssistError";
    this.status = status;
    this.code = code;
  }
}

/** Fil récent visible par l'utilisateur (participation, hiddenAt, hors supprimés et système). */
async function loadThread(
  etablissementId: string,
  conversationId: string,
  userId: string,
): Promise<AssistMessage[]> {
  const db = getDb();
  const parts = await db
    .select({ hiddenAt: messagingParticipant.hiddenAt })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.conversationId, conversationId),
        eq(messagingParticipant.userId, userId),
      ),
    )
    .limit(1);
  const part = parts[0];
  if (!part) {
    throw new AssistError("Conversation introuvable.", 404, "CONVERSATION_NOT_FOUND");
  }

  const conditions = [
    eq(messagingMessage.etablissementId, etablissementId),
    eq(messagingMessage.conversationId, conversationId),
    isNull(messagingMessage.deletedAt),
    ne(messagingMessage.type, "system"),
  ];
  if (part.hiddenAt) conditions.push(gt(messagingMessage.createdAt, part.hiddenAt));

  const rows = await db
    .select({
      senderId: messagingMessage.senderId,
      body: messagingMessage.body,
      createdAt: messagingMessage.createdAt,
      senderName: user.name,
    })
    .from(messagingMessage)
    .leftJoin(
      user,
      and(eq(user.id, messagingMessage.senderId), eq(user.etablissementId, etablissementId)),
    )
    .where(and(...conditions))
    .orderBy(desc(messagingMessage.createdAt))
    .limit(THREAD_LIMIT);

  return rows
    .reverse()
    .filter((r) => (r.body ?? "").trim())
    .map((r) => ({
      authorFirstName: (r.senderName ?? "").trim().split(/\s+/)[0] || "Collègue",
      isMine: r.senderId === userId,
      at: r.createdAt,
      body: r.body ?? "",
    }));
}

async function callMistral(
  model: string,
  messages: unknown[],
  apiKey: string,
): Promise<Response | null> {
  try {
    return await fetch(MISTRAL_URL, {
      method: "POST",
      signal: AbortSignal.timeout(CALL_TIMEOUT_MS),
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${apiKey}` },
      body: JSON.stringify({ model, messages, ...mistralModelOptions(model) }),
    });
  } catch {
    return null;
  }
}

export type AssistInput = {
  etablissementId: string;
  conversationId: string;
  userId: string;
  mode: AssistMode;
  instruction?: string;
  draft?: string;
  tone?: AssistTone;
  apiKey: string;
};

export async function runMessagingAssist(input: AssistInput): Promise<string> {
  if (input.mode === "rewrite" && !input.draft?.trim()) {
    throw new AssistError("Écrivez d'abord un brouillon à améliorer.", 400, "DRAFT_REQUIRED");
  }
  const thread = await loadThread(input.etablissementId, input.conversationId, input.userId);
  if (input.mode !== "rewrite" && thread.length === 0) {
    throw new AssistError("Aucun message dans cette conversation.", 400, "EMPTY_THREAD");
  }

  const prompt = buildAssistPrompt({
    mode: input.mode,
    thread,
    draft: input.draft,
    instruction: input.instruction,
    tone: input.tone,
  });

  // Anonymisation puis restauration des vrais noms dans la réponse.
  // Échec fermé : sans anonymisation possible, rien ne part vers le modèle.
  let userContent: string;
  let restore: (s: string) => string;
  try {
    const { pseudonymizeForLlmStrict } = await import("@/app/lib/brain-ai/pseudonymize");
    const p = await pseudonymizeForLlmStrict(input.etablissementId, [prompt.user]);
    userContent = p.texts[0] ?? "";
    restore = p.restore;
  } catch {
    throw new AssistError(
      "Assistant momentanément indisponible (anonymisation impossible). Réessayez plus tard.",
      503,
      "ASSIST_UNAVAILABLE",
    );
  }
  if (!userContent) {
    throw new AssistError("Assistant momentanément indisponible.", 503, "ASSIST_UNAVAILABLE");
  }

  const messages = [
    { role: "system", content: prompt.system },
    { role: "user", content: userContent },
  ];
  let res = await callMistral(MISTRAL_LARGE_MODEL, messages, input.apiKey);
  if (!res || (!res.ok && [429, 500, 502, 503, 504].includes(res.status))) {
    res = await callMistral(MISTRAL_SMALL_MODEL, messages, input.apiKey);
  }
  if (!res || !res.ok) {
    throw new AssistError(
      "Assistant temporairement indisponible. Réessayez dans un instant.",
      502,
      "ASSIST_UPSTREAM",
    );
  }
  const data = await res.json().catch(() => null);
  const text = restore(mistralMessageText(data?.choices?.[0]?.message?.content).trim());
  if (!text) {
    throw new AssistError("L'assistant n'a rien proposé. Réessayez.", 502, "ASSIST_EMPTY");
  }
  return text;
}
