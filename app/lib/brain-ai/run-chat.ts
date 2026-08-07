import {
  buildContextFromEntries,
  readKnowledgeDocument,
  readKnowledgeIndex,
  selectDomainByMessage,
} from "@/app/lib/knowledge";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import {
  createConversationState,
  normalizeConversationState,
  withPendingChoices,
  withPendingConfirmation,
} from "@/app/lib/brain-ai/conversation-state";
import { executeBrainTool } from "@/app/lib/brain-ai/tools/execute";
import { getBrainTool, mistralToolsForUser } from "@/app/lib/brain-ai/tools/registry";
import { detectWizardStartTool } from "@/app/lib/brain-ai/wizard-intent";
import {
  TRAVELS_CLASSES_AUTRES_LABEL,
  TRAVELS_CLASSES_AUTRES_VALUE,
} from "@/app/lib/travels-classes";
import type {
  BrainChatResponse,
  BrainCta,
  BrainConversationState,
  BrainPendingChoices,
  BrainPendingConfirmation,
  BrainToolCtx,
  BrainToolResult,
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

function addDaysToDateKey(dateKey: string, days: number): string {
  const [y, m, d] = dateKey.split("-").map(Number);
  const next = new Date(Date.UTC(y!, m! - 1, d! + days, 12, 0, 0));
  return next.toISOString().slice(0, 10);
}

function weekdayLongFr(dateKey: string): string {
  return new Intl.DateTimeFormat("fr-FR", {
    timeZone: "UTC",
    weekday: "long",
  }).format(new Date(`${dateKey}T12:00:00Z`));
}

/** Ancre calendaire pour éviter les dates hallucinées (ex. « demain = juin 2024 »). */
function buildBrainAiClockContext(now = new Date()): string {
  const today = calendarDateKeyParis(now);
  const tomorrow = addDaysToDateKey(today, 1);
  const timeFr = new Intl.DateTimeFormat("fr-FR", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
  }).format(now);
  const todayLong = new Intl.DateTimeFormat("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  }).format(now);
  return (
    `Horloge institutionnelle (fuseau Europe/Paris, source de vérité) :\n` +
    `- Maintenant : ${todayLong}, ${timeFr}.\n` +
    `- Aujourd'hui = ${today} (${weekdayLongFr(today)}).\n` +
    `- Demain = ${tomorrow} (${weekdayLongFr(tomorrow)}).\n` +
    `- Pour « lundi prochain », « dans 3 jours », etc., calcule TOUJOURS à partir de cette date — n'invente jamais une année ancienne (ex. 2024).\n` +
    `- Les outils qui demandent une date exigent le format YYYY-MM-DD basé sur cette horloge.\n`
  );
}

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

function applyChoiceToArgs(
  draftArgs: Record<string, unknown>,
  field: string,
  value?: string,
  values?: string[],
): Record<string, unknown> {
  const next = { ...draftArgs };
  if (field === "selectedHours") {
    const raw = values?.length ? values : value ? [value] : [];
    next.selectedHours = raw.map((h) => Number(h)).filter((h) => Number.isFinite(h));
    return next;
  }
  if (field === "roomId") {
    next.roomId = value ?? "";
    delete next.date;
    delete next.selectedHours;
    return next;
  }
  if (field === "date" || field === "startDate" || field === "endDate") {
    next[field] = value ?? "";
    if (field === "date" || field === "startDate") {
      next.date = value ?? "";
      next.startDate = value ?? "";
      delete next.selectedHours;
    }
    return next;
  }
  if (field === "nbEleves") {
    const n = Number(String(value || "").trim().replace(",", "."));
    if (Number.isFinite(n) && n > 0) next.nbEleves = Math.round(n);
    next.nbElevesResolved = true;
    return next;
  }
  if (field === "nombrePhotocopies" || field === "nombreHeures") {
    const n = Number(String(value || "").trim().replace(",", "."));
    if (Number.isFinite(n)) next[field] = n;
    return next;
  }
  if (field === "reasonOther") {
    next.reason = value ?? "";
    return next;
  }
  if (field === "detailsCustom") {
    next.details = value ?? "";
    next.detailsResolved = true;
    return next;
  }
  if (field === "details") {
    if (value === "Non") {
      next.details = "";
      next.detailsResolved = true;
    } else {
      next.details = value ?? "";
    }
    return next;
  }
  if (field === "pole") {
    next.pole = value ?? "";
    delete next.className;
    return next;
  }
  if (field === "classes") {
    const raw = values?.length ? values : value ? [value] : [];
    const wantsAutres = raw.some(
      (c) => c === TRAVELS_CLASSES_AUTRES_VALUE || c === TRAVELS_CLASSES_AUTRES_LABEL,
    );
    next.classes = raw.join(", ");
    if (!wantsAutres) next.classesResolved = true;
    else delete next.classesResolved;
    return next;
  }
  if (field === "classesOther") {
    const other = String(value || "").trim();
    const base = splitDraftClasses(String(next.classes || ""));
    const withoutAutres = base.filter(
      (c) => c !== TRAVELS_CLASSES_AUTRES_VALUE && c !== TRAVELS_CLASSES_AUTRES_LABEL,
    );
    next.classes = [...withoutAutres, ...(other ? [other] : [])].join(", ");
    next.classesResolved = true;
    delete next.classesOther;
    delete next.classesOtherPending;
    return next;
  }
  next[field] = value ?? (values?.length ? values.join(", ") : "");
  return next;
}

function splitDraftClasses(raw: string): string[] {
  return raw
    .split(/[,;/]+/)
    .map((c) => c.trim())
    .filter(Boolean);
}

function materializeToolTurn(
  conversationState: BrainConversationState,
  result: BrainToolResult,
  ctas: BrainCta[],
  knowledgeMeta?: { domainId?: string; file?: string },
): BrainChatResponse {
  if (!result.ok && "needsChoices" in result && result.needsChoices) {
    const pendingChoices: BrainPendingChoices = {
      tool: result.tool,
      field: result.field,
      promptFr: result.promptFr,
      options: result.options,
      draftArgs: result.draftArgs,
      selectionType: result.selectionType || "single",
    };
    const state = withPendingChoices(conversationState, pendingChoices);
    return {
      answer: result.promptFr,
      domain: knowledgeMeta?.domainId,
      usedFile: knowledgeMeta?.file,
      conversationState: state,
      pendingConfirmation: null,
      pendingChoices,
      ctas: ctas.length ? ctas : undefined,
    };
  }

  if (!result.ok && "needsConfirmation" in result && result.needsConfirmation) {
    const pendingConfirmation: BrainPendingConfirmation = {
      tool: result.tool,
      args: result.args,
      summaryFr: result.summaryFr,
    };
    const state = withPendingConfirmation(conversationState, pendingConfirmation);
    return {
      answer: `${result.summaryFr}\n\nCliquez sur Confirmer pour valider, ou annulez pour modifier.`,
      domain: knowledgeMeta?.domainId,
      usedFile: knowledgeMeta?.file,
      conversationState: state,
      pendingConfirmation,
      pendingChoices: null,
      ctas: ctas.length ? ctas : undefined,
    };
  }

  if (!result.ok) {
    return {
      answer: "error" in result ? result.error : "Échec de l'action.",
      conversationState: withPendingChoices(
        withPendingConfirmation(conversationState, null),
        null,
      ),
      pendingConfirmation: null,
      pendingChoices: null,
      ctas: ctas.length ? ctas : undefined,
    };
  }

  const nextCtas = [...ctas, ...extractCtas(result.data)];
  const follow =
    result.data && typeof result.data === "object" && "followUrl" in (result.data as object)
      ? String((result.data as { followUrl?: string }).followUrl || "")
      : "";
  if (follow && !nextCtas.some((c) => c.href === follow)) {
    nextCtas.push({ label: "Ouvrir", href: follow });
  }
  return {
    answer: result.summaryFr || "Action effectuée.",
    domain: knowledgeMeta?.domainId,
    usedFile: knowledgeMeta?.file,
    conversationState: withPendingChoices(
      withPendingConfirmation(conversationState, null),
      null,
    ),
    pendingConfirmation: null,
    pendingChoices: null,
    ctas: nextCtas.length ? nextCtas : undefined,
  };
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
  /** Réponse à une liste déroulante / choix structuré. */
  choiceApply?: {
    tool: string;
    field: string;
    value?: string;
    values?: string[];
    draftArgs: Record<string, unknown>;
  } | null;
  /** Pièces jointes déjà uploadées (PDF…). */
  attachments?: Array<{
    key: string;
    fileName: string;
    contentType?: string;
  }>;
};

export async function runBrainChat(input: RunBrainChatInput): Promise<BrainChatResponse> {
  let conversationState: BrainConversationState = normalizeConversationState(
    input.conversationState,
  );
  if (!conversationState.conversationId) {
    conversationState = createConversationState();
  }

  if (input.attachments?.length) {
    const prev = Array.isArray(conversationState.slots.attachments)
      ? (conversationState.slots.attachments as unknown[])
      : [];
    conversationState = {
      ...conversationState,
      slots: {
        ...conversationState.slots,
        attachments: [
          ...prev,
          ...input.attachments.map((a) => ({
            key: a.key,
            fileName: a.fileName,
            contentType: a.contentType || "application/pdf",
          })),
        ],
      },
    };
  }

  const ctas: BrainCta[] = [];
  let pendingConfirmation: BrainPendingConfirmation | null = null;
  let pendingChoices: BrainPendingChoices | null = null;

  // Choix UI (liste déroulante / multi / date) — rejoue l'outil sans passer par le LLM
  if (input.choiceApply?.tool && input.choiceApply.field) {
    const toolName = input.choiceApply.tool;
    const tool = getBrainTool(toolName);
    if (!tool) {
      return {
        answer: "Action inconnue, impossible d'appliquer ce choix.",
        conversationState,
        pendingConfirmation: null,
        pendingChoices: null,
      };
    }
    const mergedArgs = applyChoiceToArgs(
      input.choiceApply.draftArgs || {},
      input.choiceApply.field,
      input.choiceApply.value,
      input.choiceApply.values,
    );
    const result = await executeBrainTool(toolName, mergedArgs, {
      ...input.toolCtx,
      confirmed: false,
    });
    return materializeToolTurn(conversationState, result, ctas);
  }

  // Confirmation directe (bouton UI) — fusionne éventuelle PJ récente dans les args photocopies
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
    let confirmArgs = { ...(input.confirmAction.args || {}) };
    if (toolName === "create_photocopie_demand" && !confirmArgs.documentKey) {
      const atts = conversationState.slots.attachments;
      if (Array.isArray(atts) && atts.length > 0) {
        const last = atts[atts.length - 1] as {
          key?: string;
          fileName?: string;
          contentType?: string;
        };
        if (last?.key) {
          confirmArgs = {
            ...confirmArgs,
            documentKey: last.key,
            documentFileName: last.fileName || "document.pdf",
            documentContentType: last.contentType || "application/pdf",
          };
        }
      }
    }
    const result = await executeBrainTool(toolName, confirmArgs, {
      ...input.toolCtx,
      confirmed: true,
    });
    conversationState = withPendingConfirmation(conversationState, null);
    conversationState = withPendingChoices(conversationState, null);
    if (!result.ok) {
      return {
        answer: "error" in result ? result.error : "Échec de l'action.",
        conversationState,
        pendingConfirmation: null,
        pendingChoices: null,
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
      pendingChoices: null,
      ctas: ctas.length ? ctas : undefined,
    };
  }

  const signedIn = Boolean(input.toolCtx.userId) && input.audience === "private";

  // Bypass LLM : intention d'action → wizard UI tout de suite (select / boutons).
  if (signedIn) {
    const wizardTool = detectWizardStartTool(input.message);
    if (wizardTool && getBrainTool(wizardTool)) {
      const result = await executeBrainTool(wizardTool, {}, {
        ...input.toolCtx,
        confirmed: false,
      });
      return materializeToolTurn(conversationState, result, ctas);
    }
  }

  if (!input.apiKey) {
    return {
      answer: "Le service IA n'est pas configuré (MISTRAL_API_KEY).",
      conversationState,
      pendingConfirmation: null,
      pendingChoices: null,
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

  const tools = mistralToolsForUser(signedIn);

  const systemPrompt =
    `Tu es ScolIA, l'assistant institutionnel de l'établissement (Brain AI).\n` +
    `Réponds en français, précis, utile et concis.\n` +
    buildBrainAiClockContext() +
    `Tu as deux sources d'information :\n` +
    `1) Dictionnaire (contexte knowledge ci-dessous) — infos stables (FAQ, circulaires…).\n` +
    `2) Actualité live via outils (feuille de semaine, voyages, salles, photocopies, HSE, stages, internat…) — toujours préférer un outil pour l'actualité.\n` +
    `Règles STRICTES (actions) :\n` +
    `- INTERDIT de demander en texte libre la salle, la date, les créneaux, le motif, etc.\n` +
    `- INTERDIT d'écrire « dites-moi… », « pour commencer… », « liste-moi les salles… ».\n` +
    `- Dès que l'utilisateur veut réserver / créer / déclarer : appelle IMMÉDIATEMENT l'outil correspondant AVEC {} (sans args). L'UI affiche listes déroulantes, dates et boutons.\n` +
    `- create_reservation = réservation salle | create_trip = sortie/voyage | create_request = demande | create_absence = absence | create_photocopie_demand | create_hse_demand.\n` +
    `- Pas d'accès RH / dossiers personnels / salaires. create_absence = soi uniquement.\n` +
    `- Si un PDF est joint, passe-le à create_photocopie_demand (documentKey / documentFileName).\n` +
    `- Si needsConfirmation : présente uniquement le récap (l'UI a Confirmer / Modifier / Annuler).\n` +
    `- N'invente pas : si l'info manque après les outils, dis-le clairement.\n` +
    `- Liens en URL complète https://…\n` +
    `Séjours scolaires (travels) :\n` +
    `- SIMPLE ≠ COMPLEX : SIMPLE n'a pas d'étape devis bus ; COMPLEX avec needsBus=true a Logistique puis Signature.\n` +
    `- À PROF_LOGISTICS : créateur peut « Choisir » un devis ; direction peut « Choisir et signer ».\n` +
    `- Si l'utilisateur demande où en est un séjour / quoi faire / ce qui bloque : appelle get_trip_status (tripId) et base-toi sur audit / auditText / projectSnapshot.\n` +
    `- Conseille concrètement (attendre vs choisir des devis, montants manquants en compta, etc.) sans inventer des champs absents.\n`;

  const attachmentNote = (() => {
    const atts = conversationState.slots.attachments;
    if (!Array.isArray(atts) || atts.length === 0) return "";
    return (
      `\nPièces jointes disponibles dans cette conversation:\n` +
      atts
        .map((a, i) => {
          const row = a as { key?: string; fileName?: string; contentType?: string };
          return `- [${i + 1}] ${row.fileName || "fichier"} (key=${row.key}, type=${row.contentType || "application/pdf"})`;
        })
        .join("\n") +
      `\n`
    );
  })();

  const historyText = input.history
    .slice(-12)
    .map((m) => `${m.role === "user" ? "Utilisateur" : "Assistant"}: ${m.content}`)
    .join("\n");

  const userPayload =
    `Historique récent:\n${historyText || "(aucun)"}\n\n` +
    `Contexte dictionnaire:\n${knowledge.context}\n` +
    attachmentNote +
    `\nQuestion: ${input.message}`;

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
      conversationState = withPendingChoices(conversationState, pendingChoices);
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
        pendingChoices,
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
      let args = parseToolArgs(call.function?.arguments || "{}");
      if (name === "create_photocopie_demand" && !args.documentKey) {
        const atts = conversationState.slots.attachments;
        if (Array.isArray(atts) && atts.length > 0) {
          const last = atts[atts.length - 1] as {
            key?: string;
            fileName?: string;
            contentType?: string;
          };
          if (last?.key) {
            args = {
              ...args,
              documentKey: last.key,
              documentFileName: last.fileName || "document.pdf",
              documentContentType: last.contentType || "application/pdf",
            };
          }
        }
      }
      const result = await executeBrainTool(name, args, {
        ...input.toolCtx,
        confirmed: false,
      });

      if (!result.ok && "needsChoices" in result && result.needsChoices) {
        return materializeToolTurn(
          conversationState,
          result,
          ctas,
          { domainId: knowledge.domain.id, file: knowledge.domain.file },
        );
      }

      if (!result.ok && "needsConfirmation" in result && result.needsConfirmation) {
        return materializeToolTurn(
          conversationState,
          result,
          ctas,
          { domainId: knowledge.domain.id, file: knowledge.domain.file },
        );
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
  conversationState = withPendingChoices(conversationState, pendingChoices);
  return {
    answer:
      "J'ai atteint la limite d'actions pour ce tour. Reformulez ou confirmez l'action proposée.",
    conversationState,
    pendingConfirmation,
    pendingChoices,
    ctas: ctas.length ? ctas : undefined,
  };
}
