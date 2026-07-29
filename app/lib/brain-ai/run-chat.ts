import {
  buildContextFromEntries,
  readKnowledgeDocument,
  readKnowledgeIndex,
  selectDomainByMessage,
} from "@/app/lib/knowledge";
import {
  createConversationState,
  normalizeConversationState,
  withPendingConfirmation,
} from "@/app/lib/brain-ai/conversation-state";
import { executeBrainTool } from "@/app/lib/brain-ai/tools/execute";
import { getBrainTool, mistralToolsForUser } from "@/app/lib/brain-ai/tools/registry";
import type {
  BrainChatResponse,
  BrainCta,
  BrainConversationState,
  BrainPendingConfirmation,
  BrainToolCtx,
} from "@/app/lib/brain-ai/types";

const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";
const MAX_TOOL_ROUNDS = 5;

type ChatMessage = {
  role: "system" | "user" | "assistant" | "tool";
  content: string | null;
  tool_calls?: Array<{
    id: string;
    type: "function";
    function: { name: string; arguments: string };
  }>;
  tool_call_id?: string;
  name?: string;
};

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function normalizeLinks(text: string) {
  return text.replace(/\bwww\.[^\s<>"')\]]+/gi, (raw) => `https://${raw}`);
}

async function fetchMistralWithRetry(body: unknown, apiKey: string, attempts = 3) {
  let lastResponse: Response | null = null;
  for (let i = 0; i < attempts; i += 1) {
    const res = await fetch(MISTRAL_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify(body),
    });
    if (res.ok) return res;
    lastResponse = res;
    if (![429, 500, 502, 503, 504].includes(res.status) || i === attempts - 1) {
      return res;
    }
    await sleep(350 * (i + 1));
  }
  return lastResponse;
}

async function classifyDomainWithMistral(
  message: string,
  domains: Array<{ id: string; label: string }>,
  apiKey: string,
) {
  const domainList = domains.map((d) => `- ${d.id}: ${d.label}`).join("\n");
  const prompt =
    `Tu dois classer une question utilisateur dans UN SEUL domaine.\n` +
    `Réponds uniquement en JSON: {"domainId":"..."}\n` +
    `Domaine possibles:\n${domainList}\n\n` +
    `Question:\n${message}`;
  const res = await fetchMistralWithRetry(
    {
      model: "mistral-small-latest",
      temperature: 0,
      response_format: { type: "json_object" },
      messages: [{ role: "user", content: prompt }],
    },
    apiKey,
  );
  if (!res?.ok) return null;
  const data = await res.json();
  try {
    const parsed = JSON.parse(data?.choices?.[0]?.message?.content ?? "{}") as {
      domainId?: string;
    };
    return typeof parsed.domainId === "string" ? parsed.domainId.trim() : null;
  } catch {
    return null;
  }
}

async function buildKnowledgeContext(
  message: string,
  audience: "public" | "private",
  apiKey: string | null,
) {
  const index = await readKnowledgeIndex();
  let domain = selectDomainByMessage(index.domains, message);
  const selectedByKeywords = domain;
  const mistralDomainId = apiKey
    ? await classifyDomainWithMistral(
        message,
        index.domains.map((d) => ({ id: d.id, label: d.label })),
        apiKey,
      )
    : null;
  if (mistralDomainId) {
    const found = index.domains.find((d) => d.id === mistralDomainId);
    if (found) domain = found;
  }

  const text = message.toLowerCase();
  const keywordRanked = index.domains
    .map((d) => ({
      domain: d,
      score: d.keywords.reduce((acc, kw) => (text.includes(kw.toLowerCase()) ? acc + 1 : acc), 0),
    }))
    .sort((a, b) => b.score - a.score)
    .map((x) => x.domain);

  const selectedDomains: typeof index.domains = [];
  const pushUnique = (d?: (typeof index.domains)[number]) => {
    if (!d) return;
    if (selectedDomains.some((x) => x.id === d.id)) return;
    selectedDomains.push(d);
  };
  pushUnique(domain);
  pushUnique(keywordRanked[0]);
  pushUnique(keywordRanked[1]);
  pushUnique(keywordRanked[2]);
  const finalDomains = selectedDomains.slice(0, 3);
  const docs = await Promise.all(finalDomains.map((d) => readKnowledgeDocument(d.file)));
  const context = docs
    .map(
      (doc, i) =>
        `### Domaine: ${finalDomains[i].label}\n${buildContextFromEntries(finalDomains[i], doc, audience, 8, message, 90)}`,
    )
    .join("\n\n");

  return {
    domain,
    selectedByKeywords,
    finalDomains,
    context,
  };
}

function parseToolArgs(raw: string): Record<string, unknown> {
  try {
    const parsed = JSON.parse(raw || "{}");
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : {};
  } catch {
    return {};
  }
}

function extractCtas(data: unknown): BrainCta[] {
  if (!data || typeof data !== "object") return [];
  const ctas = (data as { ctas?: unknown }).ctas;
  if (!Array.isArray(ctas)) return [];
  return ctas
    .filter((c): c is BrainCta => Boolean(c && typeof c === "object" && typeof (c as BrainCta).href === "string"))
    .map((c) => ({ label: String((c as BrainCta).label || "Ouvrir"), href: (c as BrainCta).href }));
}

export type RunBrainChatInput = {
  message: string;
  audience: "public" | "private";
  history: Array<{ role: "user" | "assistant"; content: string }>;
  apiKey: string | null;
  toolCtx: BrainToolCtx;
  conversationState?: unknown;
  /** Confirmation explicite d'une action mutante. */
  confirm?: boolean;
  confirmAction?: { tool: string; args: Record<string, unknown> } | null;
};

export async function runBrainChat(input: RunBrainChatInput): Promise<BrainChatResponse> {
  let conversationState: BrainConversationState = normalizeConversationState(
    input.conversationState,
  );
  if (!conversationState.conversationId) {
    conversationState = createConversationState();
  }

  const ctas: BrainCta[] = [];
  let pendingConfirmation: BrainPendingConfirmation | null = null;

  // Confirmation directe (bouton UI)
  if (input.confirm && input.confirmAction?.tool) {
    const toolName = input.confirmAction.tool;
    const tool = getBrainTool(toolName);
    if (!tool) {
      return {
        answer: "Action inconnue, impossible de confirmer.",
        conversationState,
        pendingConfirmation: null,
      };
    }
    const result = await executeBrainTool(toolName, input.confirmAction.args || {}, {
      ...input.toolCtx,
      confirmed: true,
    });
    conversationState = withPendingConfirmation(conversationState, null);
    if (!result.ok) {
      return {
        answer: "error" in result ? result.error : "Échec de l'action.",
        conversationState,
        pendingConfirmation: null,
      };
    }
    ctas.push(...extractCtas(result.data));
    const follow =
      result.data && typeof result.data === "object" && "followUrl" in (result.data as object)
        ? String((result.data as { followUrl?: string }).followUrl || "")
        : "";
    if (follow) ctas.push({ label: "Ouvrir", href: follow });
    return {
      answer: result.summaryFr || "Action effectuée.",
      conversationState,
      pendingConfirmation: null,
      ctas: ctas.length ? ctas : undefined,
    };
  }

  if (!input.apiKey) {
    return {
      answer: "Le service IA n'est pas configuré (MISTRAL_API_KEY).",
      conversationState,
      pendingConfirmation: null,
    };
  }

  let knowledge;
  try {
    knowledge = await buildKnowledgeContext(input.message, input.audience, input.apiKey);
  } catch (err) {
    console.warn("[brain-ai] knowledge unavailable", err);
    knowledge = {
      domain: { id: "none", label: "Aucun", file: "", isYearlyReset: false, keywords: [] },
      selectedByKeywords: { id: "none", label: "Aucun", file: "", isYearlyReset: false, keywords: [] },
      finalDomains: [] as Array<{ id: string; file: string; label: string }>,
      context: "(base de connaissances indisponible)",
    };
  }

  const signedIn = Boolean(input.toolCtx.userId) && input.audience === "private";
  const tools = mistralToolsForUser(signedIn);

  const systemPrompt =
    `Tu es Nico, l'assistant institutionnel de l'établissement (Brain AI).\n` +
    `Réponds en français, précis, utile et concis.\n` +
    `Tu as deux sources d'information :\n` +
    `1) Dictionnaire (contexte knowledge ci-dessous) — infos stables (FAQ, circulaires…).\n` +
    `2) Actualité live via outils (feuille de semaine, voyages, salles, photocopies, HSE, stages, internat…) — toujours préférer un outil pour l'actualité.\n` +
    `Règles:\n` +
    `- Pas d'accès RH / dossiers personnels / salaires. create_absence = soi uniquement. HSE = soi ou direction établissement.\n` +
    `- Pour une action mutante (réservation, demande, absence, séjour, photocopies, HSE), collecter les champs manquants puis appeler l'outil (il demandera confirmation).\n` +
    `- Si un outil renvoie needsConfirmation, présente le résumé et demande à l'utilisateur de confirmer (bouton).\n` +
    `- N'invente pas : si l'info manque, dis-le clairement.\n` +
    `- Liens en URL complète https://…\n`;

  const historyText = input.history
    .slice(-12)
    .map((m) => `${m.role === "user" ? "Utilisateur" : "Assistant"}: ${m.content}`)
    .join("\n");

  const userPayload =
    `Historique récent:\n${historyText || "(aucun)"}\n\n` +
    `Contexte dictionnaire:\n${knowledge.context}\n\n` +
    `Question: ${input.message}`;

  const messages: ChatMessage[] = [
    { role: "system", content: systemPrompt },
    { role: "user", content: userPayload },
  ];

  for (let round = 0; round < MAX_TOOL_ROUNDS; round += 1) {
    const body: Record<string, unknown> = {
      model: "mistral-small-latest",
      temperature: 0.2,
      messages,
    };
    if (tools.length > 0) {
      body.tools = tools;
      body.tool_choice = "auto";
    }

    const llm = await fetchMistralWithRetry(body, input.apiKey);
    if (!llm) {
      return {
        answer: "Le service IA est temporairement indisponible. Réessaie dans quelques secondes.",
        conversationState,
        pendingConfirmation: null,
      };
    }
    if (!llm.ok) {
      if ([429, 500, 502, 503, 504].includes(llm.status)) {
        return {
          answer: "Le service IA est temporairement indisponible. Réessaie dans quelques secondes.",
          conversationState,
          pendingConfirmation: null,
        };
      }
      const err = await llm.text();
      return {
        answer: `Erreur Mistral: ${err}`,
        conversationState,
        pendingConfirmation: null,
      };
    }

    const data = await llm.json();
    const msg = data?.choices?.[0]?.message as ChatMessage | undefined;
    const toolCalls = msg?.tool_calls;

    if (!toolCalls?.length) {
      const answer = normalizeLinks(msg?.content?.trim() || "");
      conversationState = withPendingConfirmation(conversationState, pendingConfirmation);
      return {
        answer: answer || "Je n'ai pas pu formuler de réponse pour le moment.",
        domain: knowledge.domain.id,
        usedFile: knowledge.domain.file,
        usedDomains: knowledge.finalDomains.map((d) => ({
          id: d.id,
          file: d.file,
          label: d.label,
        })),
        fallbackFrom:
          knowledge.selectedByKeywords.id !== knowledge.domain.id
            ? knowledge.selectedByKeywords.id
            : undefined,
        conversationState,
        pendingConfirmation,
        ctas: ctas.length ? ctas : undefined,
      };
    }

    messages.push({
      role: "assistant",
      content: msg?.content ?? null,
      tool_calls: toolCalls,
    });

    for (const call of toolCalls) {
      const name = call.function?.name || "";
      const args = parseToolArgs(call.function?.arguments || "{}");
      const result = await executeBrainTool(name, args, {
        ...input.toolCtx,
        confirmed: false,
      });

      if (!result.ok && "needsConfirmation" in result && result.needsConfirmation) {
        pendingConfirmation = {
          tool: result.tool,
          args: result.args,
          summaryFr: result.summaryFr,
        };
        conversationState = withPendingConfirmation(conversationState, pendingConfirmation);
        return {
          answer: `${result.summaryFr}\n\nCliquez sur Confirmer pour valider, ou annulez pour modifier.`,
          domain: knowledge.domain.id,
          usedFile: knowledge.domain.file,
          conversationState,
          pendingConfirmation,
          ctas: ctas.length ? ctas : undefined,
        };
      }

      if (result.ok) {
        ctas.push(...extractCtas(result.data));
        const follow =
          result.data && typeof result.data === "object" && "followUrl" in (result.data as object)
            ? String((result.data as { followUrl?: string }).followUrl || "")
            : "";
        if (follow && !ctas.some((c) => c.href === follow)) {
          ctas.push({ label: "Ouvrir", href: follow });
        }
      }

      messages.push({
        role: "tool",
        tool_call_id: call.id,
        name,
        content: JSON.stringify(result),
      });
    }
  }

  conversationState = withPendingConfirmation(conversationState, pendingConfirmation);
  return {
    answer:
      "J'ai atteint la limite d'actions pour ce tour. Reformulez ou confirmez l'action proposée.",
    conversationState,
    pendingConfirmation,
    ctas: ctas.length ? ctas : undefined,
  };
}
