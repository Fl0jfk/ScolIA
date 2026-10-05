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
  withPendingFileUpload,
} from "@/app/lib/brain-ai/conversation-state";
import { executeBrainTool } from "@/app/lib/brain-ai/tools/execute";
import { getBrainTool, mistralToolsForUser } from "@/app/lib/brain-ai/tools/registry";
import { isBrainPermissionDenied } from "@/app/lib/brain-ai/permissions";
import { loadScoliaPersonalSignalsBrief } from "@/app/lib/brain-ai/personal-signals";
import { detectWizardStartTool } from "@/app/lib/brain-ai/wizard-intent";
import {
  TRAVELS_CLASSES_AUTRES_LABEL,
  TRAVELS_CLASSES_AUTRES_VALUE,
} from "@/app/lib/travels-classes";
import type {
  BrainChatResponse,
  BrainClientAction,
  BrainCta,
  BrainConversationState,
  BrainDocCatalog,
  BrainDocCatalogGroup,
  BrainDocCatalogItem,
  BrainPendingChoices,
  BrainPendingConfirmation,
  BrainPendingFileUpload,
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

/** Liens proxy PDF dossier élève : inutilisables en `<a>` (besoin de la modale aperçu). */
const ELEVE_DOC_FILE_PATH =
  /\/api\/eleves\/[^)\s<>"']+\/documents\/[^)\s<>"']+\/file\/?[^)\s<>"']*/gi;

/**
 * Retire du texte les liens vers les pièces (markdown ou URL nues).
 * Les CTAs UI ouvrent ces documents correctement — éviter le doublon cassé dans le chat.
 */
function stripEleveDocumentLinks(text: string): string {
  let out = text.replace(
    /\[([^\]]+)\]\(\s*\/api\/eleves\/[^)]+\/documents\/[^)]+\/file\/?[^)]*\)/gi,
    "",
  );
  out = out.replace(ELEVE_DOC_FILE_PATH, "");
  // Nettoyage lignes / puces laissées vides par les suppressions
  out = out
    .replace(/^[ \t]*[-•*]\s*$/gm, "")
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .replace(/[ \t]{2,}/g, " ")
    .trim();
  return out;
}

/** Outils dont la réponse structurée (summaryFr + ctas) doit primer sur le LLM. */
function shouldMaterializeStructuredList(toolName: string, result: BrainToolResult): boolean {
  if (!result.ok) return false;
  if (toolName === "list_eleves_filtered") return true;
  if (toolName === "open_eleve_dossier") {
    const ctas = extractCtas(result.data);
    return ctas.some((c) => c.preview);
  }
  return false;
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
    .map((c) => ({
      label: String((c as BrainCta).label || "Ouvrir"),
      href: (c as BrainCta).href,
      ...((c as BrainCta).preview ? { preview: true as const } : {}),
      ...((c as BrainCta).subtitle ? { subtitle: String((c as BrainCta).subtitle) } : {}),
      ...((c as BrainCta).group ? { group: String((c as BrainCta).group) } : {}),
    }));
}

function extractDocCatalog(data: unknown): BrainDocCatalog | undefined {
  if (!data || typeof data !== "object") return undefined;
  const raw = (data as { docCatalog?: unknown }).docCatalog;
  if (!raw || typeof raw !== "object") return undefined;
  const catalog = raw as BrainDocCatalog;
  if (!Array.isArray(catalog.groups)) return undefined;
  const groups: BrainDocCatalogGroup[] = [];
  for (const g of catalog.groups) {
    if (!g || typeof g !== "object" || !Array.isArray(g.items)) continue;
    const items: BrainDocCatalogItem[] = [];
    for (const item of g.items) {
      if (!item || typeof item !== "object") continue;
      const href = String(item.href || "").trim();
      const title = String(item.title || "").trim();
      if (!href || !title) continue;
      items.push({
        title,
        href,
        ...(item.subtitle ? { subtitle: String(item.subtitle) } : {}),
        ...(item.preview ? { preview: true as const } : {}),
        ...(item.dossierHref ? { dossierHref: String(item.dossierHref) } : {}),
        ...(item.ext ? { ext: String(item.ext) } : {}),
      });
    }
    if (items.length === 0) continue;
    groups.push({
      title: String(g.title || "—").trim() || "—",
      count: Number.isFinite(g.count) ? Number(g.count) : items.length,
      items,
    });
  }
  if (groups.length === 0) return undefined;
  const total = Number.isFinite(catalog.total)
    ? Number(catalog.total)
    : groups.reduce((acc, g) => acc + g.count, 0);
  return {
    title: String(catalog.title || "").trim() || `${total} document(s)`,
    ...(catalog.kindLabel ? { kindLabel: String(catalog.kindLabel) } : {}),
    total,
    groups,
  };
}

function extractClientActions(data: unknown): BrainClientAction[] {
  if (!data || typeof data !== "object") return [];
  const raw = (data as { clientActions?: unknown }).clientActions;
  if (!Array.isArray(raw)) return [];
  const out: BrainClientAction[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const type = String((item as { type?: string }).type || "");
    if (type === "open_route") {
      const href = String((item as { href?: string }).href || "").trim();
      if (!href.startsWith("/")) continue;
      out.push({
        type: "open_route",
        href,
        label: String((item as { label?: string }).label || "Ouvrir"),
      });
      continue;
    }
    if (type === "open_eleve_dossier") {
      const eleveId = String((item as { eleveId?: string }).eleveId || "").trim();
      if (!eleveId) continue;
      const subView =
        String((item as { subView?: string }).subView || "") === "inscription"
          ? "inscription"
          : "dossier";
      out.push({ type: "open_eleve_dossier", eleveId, subView });
      continue;
    }
    if (type === "open_url_modal") {
      const href = String((item as { href?: string }).href || "").trim();
      if (!href.startsWith("/")) continue;
      out.push({
        type: "open_url_modal",
        href,
        title: String((item as { title?: string }).title || ""),
      });
    }
  }
  return out;
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
  clientActions: BrainClientAction[] = [],
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
      pendingFileUpload: null,
      ctas: ctas.length ? ctas : undefined,
      clientActions: clientActions.length ? clientActions : undefined,
    };
  }

  if (!result.ok && "needsFileUpload" in result && result.needsFileUpload) {
    const pendingFileUpload: BrainPendingFileUpload = {
      tool: result.tool,
      promptFr: result.promptFr,
      draftArgs: result.draftArgs,
      optional: result.optional,
      accept: result.accept,
    };
    const state = withPendingFileUpload(conversationState, pendingFileUpload);
    return {
      answer: result.promptFr,
      domain: knowledgeMeta?.domainId,
      usedFile: knowledgeMeta?.file,
      conversationState: state,
      pendingConfirmation: null,
      pendingChoices: null,
      pendingFileUpload,
      ctas: ctas.length ? ctas : undefined,
      clientActions: clientActions.length ? clientActions : undefined,
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
      pendingFileUpload: null,
      ctas: ctas.length ? ctas : undefined,
      clientActions: clientActions.length ? clientActions : undefined,
    };
  }

  if (!result.ok) {
    return {
      answer: "error" in result ? result.error : "Échec de l'action.",
      conversationState: withPendingFileUpload(
        withPendingChoices(withPendingConfirmation(conversationState, null), null),
        null,
      ),
      pendingConfirmation: null,
      pendingChoices: null,
      pendingFileUpload: null,
      ctas: ctas.length ? ctas : undefined,
      clientActions: clientActions.length ? clientActions : undefined,
    };
  }

  const nextCtas = [...ctas, ...extractCtas(result.data)];
  const nextActions = [...clientActions, ...extractClientActions(result.data)];
  const docCatalog = extractDocCatalog(result.data);
  const follow =
    result.data && typeof result.data === "object" && "followUrl" in (result.data as object)
      ? String((result.data as { followUrl?: string }).followUrl || "")
      : "";
  if (follow && !nextCtas.some((c) => c.href === follow)) {
    nextCtas.push({ label: "Ouvrir", href: follow });
  }
  return {
    answer: stripEleveDocumentLinks(result.summaryFr || "Action effectuée."),
    domain: knowledgeMeta?.domainId,
    usedFile: knowledgeMeta?.file,
    conversationState: withPendingFileUpload(
      withPendingChoices(withPendingConfirmation(conversationState, null), null),
      null,
    ),
    pendingConfirmation: null,
    pendingChoices: null,
    pendingFileUpload: null,
    ctas: nextCtas.length ? nextCtas : undefined,
    ...(docCatalog ? { docCatalog } : {}),
    clientActions: nextActions.length ? nextActions : undefined,
  };
}

type RunBrainChatInput = {
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
  /** Reprise après demande de dépôt de fichier. */
  fileApply?: {
    tool: string;
    draftArgs: Record<string, unknown>;
    skipPdf?: boolean;
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
  const clientActions: BrainClientAction[] = [];
  let pendingConfirmation: BrainPendingConfirmation | null = null;
  let pendingChoices: BrainPendingChoices | null = null;

  // Reprise après dépôt PDF (ou skip)
  if (input.fileApply?.tool) {
    const toolName = input.fileApply.tool;
    const tool = getBrainTool(toolName);
    if (!tool) {
      return {
        answer: "Action inconnue, impossible d'appliquer le fichier.",
        conversationState,
        pendingConfirmation: null,
        pendingChoices: null,
        pendingFileUpload: null,
      };
    }
    const mergedArgs: Record<string, unknown> = {
      ...(input.fileApply.draftArgs || {}),
      ...(input.fileApply.skipPdf ? { skipPdf: true } : {}),
    };
    if (toolName === "create_photocopie_demand" && !input.fileApply.skipPdf) {
      const atts = conversationState.slots.attachments;
      if (Array.isArray(atts) && atts.length > 0) {
        const docs = atts
          .slice(-5)
          .map((a) => {
            const row = a as { key?: string; fileName?: string; contentType?: string };
            if (!row?.key) return null;
            return {
              key: row.key,
              fileName: row.fileName || "document.pdf",
              contentType: row.contentType || "application/pdf",
            };
          })
          .filter((d): d is { key: string; fileName: string; contentType: string } => Boolean(d));
        if (docs.length > 0) {
          mergedArgs.documents = docs;
          mergedArgs.documentKey = docs[0].key;
          mergedArgs.documentFileName = docs[0].fileName;
          mergedArgs.documentContentType = docs[0].contentType;
        }
      }
    }
    const result = await executeBrainTool(toolName, mergedArgs, {
      ...input.toolCtx,
      confirmed: false,
    });
    return materializeToolTurn(conversationState, result, ctas, undefined, clientActions);
  }

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
    return materializeToolTurn(conversationState, result, ctas, undefined, clientActions);
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
    if (toolName === "create_photocopie_demand") {
      const hasDocs =
        (Array.isArray(confirmArgs.documents) && confirmArgs.documents.length > 0) ||
        Boolean(confirmArgs.documentKey);
      if (!hasDocs) {
        const atts = conversationState.slots.attachments;
        if (Array.isArray(atts) && atts.length > 0) {
          const docs = atts
            .slice(-5)
            .map((a) => {
              const row = a as { key?: string; fileName?: string; contentType?: string };
              if (!row?.key) return null;
              return {
                key: row.key,
                fileName: row.fileName || "document.pdf",
                contentType: row.contentType || "application/pdf",
              };
            })
            .filter((d): d is { key: string; fileName: string; contentType: string } => Boolean(d));
          if (docs.length > 0) {
            confirmArgs = {
              ...confirmArgs,
              documents: docs,
              documentKey: docs[0].key,
              documentFileName: docs[0].fileName,
              documentContentType: docs[0].contentType,
            };
          }
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
    clientActions.push(...extractClientActions(result.data));
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
      clientActions: clientActions.length ? clientActions : undefined,
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

  const tools = mistralToolsForUser(signedIn, input.toolCtx);

  const personalSignals = signedIn
    ? await loadScoliaPersonalSignalsBrief(input.toolCtx)
    : { brief: "", items: [], source: "empty" as const };
  const personalBlock = personalSignals.brief
    ? `\nContexte personnel (signaux intranet de cet utilisateur) :\n${personalSignals.brief}\n` +
      `- Si l’utilisateur dit bonjour / « qu’est-ce que j’ai à faire » / « mes signatures » : mentionne ces points et propose d’ouvrir le lien ou d’agir (outil adapté).\n` +
      `- Pour rafraîchir ou détailler : appelle get_my_pending_actions.\n` +
      `- N’invente pas d’autres tâches hors cette liste / hors outils.\n`
    : signedIn
      ? `\nContexte personnel : aucun signal chargé pour l’instant — si on demande « à faire / signatures / file », appelle get_my_pending_actions.\n`
      : "";

  const systemPrompt =
    `Tu es ScolIA, l'assistant institutionnel de l'établissement (Brain AI).\n` +
    `Réponds en français, précis, utile et concis.\n` +
    (input.toolCtx.firstName
      ? `L’utilisateur s’appelle ${input.toolCtx.firstName} — tu peux l’appeler par son prénom, ton professionnel.\n`
      : "") +
    buildBrainAiClockContext() +
    personalBlock +
    `Tu as deux sources d'information :\n` +
    `1) Dictionnaire (contexte knowledge ci-dessous) — infos stables (FAQ, circulaires…).\n` +
    `2) Actualité live via outils (feuille de semaine, voyages, salles, photocopies, HSE, stages, internat, file personnelle…) — toujours préférer un outil pour l'actualité.\n` +
    `Droits d'accès (OBLIGATOIRE) :\n` +
    `- Tu n'as accès QU'AUX OUTILS listés dans cet appel. Ce filtre = les droits intranet de l'utilisateur.\n` +
    `- Si l'utilisateur demande une action absente de ta liste d'outils : refuse clairement. Formulation type : « Vous n'êtes pas autorisé à effectuer cette action. Elle est restreinte selon votre profil — ScolIA ne peut pas contourner vos droits. »\n` +
    `- INTERDIT d'inventer un contournement, de simuler le résultat, de « faire comme si », ou de donner des étapes pour passer outre.\n` +
    `- Si un outil renvoie FORBIDDEN / MODULE_FORBIDDEN : reprends le message d'erreur tel quel, sans l'adoucir ni proposer de bypass.\n` +
    `Règles STRICTES (actions) :\n` +
    `- INTERDIT de demander en texte libre la salle, la date, les créneaux, le motif, etc.\n` +
    `- INTERDIT d'écrire « dites-moi… », « pour commencer… », « liste-moi les salles… ».\n` +
    `- Dès que l'utilisateur veut réserver / créer / déclarer : appelle IMMÉDIATEMENT l'outil correspondant AVEC {} (sans args). L'UI affiche listes déroulantes, dates et boutons.\n` +
    `- OUVRIR ≠ CRÉER (critique) :\n` +
    `  · « ouvre / ouvrir / montre / affiche / va sur / accède » une sortie, un séjour, un voyage → open_trip (JAMAIS create_trip).\n` +
    `  · « ouvre les sorties / module voyages » sans nom → open_trip avec {} ou resolve_and_open.\n` +
    `  · « crée / créer / nouvelle / démarrer » une sortie → create_trip.\n` +
    `  · Même règle pour les autres modules : ouvrir un dossier → open_eleve_dossier ; créer un élève → create_eleve_preinscrit.\n` +
    `- File perso : get_my_pending_actions (signaux à traiter, signatures, validations).\n` +
    `- Classes : « 6ème A » / « sixième A » = 6A (pas 6E). Toujours garder la lettre de division.\n` +
    `- create_reservation = réservation salle | create_trip = NOUVELLE sortie uniquement | create_request = demande | create_absence = absence | create_photocopie_demand | create_hse_demand.\n` +
    `- Navigation : resolve_and_open | open_eleve_dossier | open_trip | search_eleves | list_eleves_filtered (PAP/classe/pôle).\n` +
    `- list_eleves_filtered : pour « tous les PAP du collège / d’une classe », appeler l’outil. L’UI affiche le catalogue complet groupé par classe — ne pas tronquer la liste dans le texte, juste confirmer le total et renvoyer vers les cartes.\n` +
    `- Mutations : update_eleve_regime | update_eleve_grille_repas | create_eleve_preinscrit | create_accueil_absence | cancel_accueil_absence | create_absence (soi) | decide_rh_absence | create_photocopie_demand | create_reservation | create_request | create_trip | create_hse_demand | assign_internat_room | resend_stage_signatures.\n` +
    `- Internat : get_internat_status | open_internat_appel | assign_internat_room.\n` +
    `- Stages : get_stages_overview | resend_stage_signatures (relance e-mails / ouvrir convention).\n` +
    `- RH absences (direction) : decide_rh_absence avec {} pour la file à valider.\n` +
    `- Voyages : open_trip (ouvrir / lister existants) | list_trips_brief | get_trip_status | create_trip (créer neuf seulement).\n` +
    `- create_absence = soi uniquement. create_accueil_absence = élèves (accueil). decide_rh_absence = file direction/validateur.\n` +
    `- Photocopies : après les champs, l'UI demande le PDF (dépôt). Ne demande pas le PDF en texte libre.\n` +
    `- Si needsConfirmation : présente uniquement le récap (l'UI a Confirmer / Modifier / Annuler).\n` +
    `- N'invente pas : si l'info manque après les outils, dis-le clairement.\n` +
    `- Liens en URL complète https://…\n` +
    `- Pièces PAP/PAI/PPS/GEVASCO : INTERDIT de coller des liens /api/eleves/…/documents/…/file dans le texte. L’UI affiche le catalogue / les boutons d’ouverture. Dans le texte, cite le total et éventuellement le détail par classe (noms), sans liens.\n` +
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
      const answer = stripEleveDocumentLinks(normalizeLinks(msg?.content?.trim() || ""));
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
        clientActions: clientActions.length ? clientActions : undefined,
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
      if (name === "create_photocopie_demand") {
        const hasDocs =
          (Array.isArray(args.documents) && args.documents.length > 0) || Boolean(args.documentKey);
        if (!hasDocs) {
          const atts = conversationState.slots.attachments;
          if (Array.isArray(atts) && atts.length > 0) {
            const docs = atts
              .slice(-5)
              .map((a) => {
                const row = a as { key?: string; fileName?: string; contentType?: string };
                if (!row?.key) return null;
                return {
                  key: row.key,
                  fileName: row.fileName || "document.pdf",
                  contentType: row.contentType || "application/pdf",
                };
              })
              .filter((d): d is { key: string; fileName: string; contentType: string } => Boolean(d));
            if (docs.length > 0) {
              args = {
                ...args,
                documents: docs,
                documentKey: docs[0].key,
                documentFileName: docs[0].fileName,
                documentContentType: docs[0].contentType,
              };
            }
          }
        }
      }
      const result = await executeBrainTool(name, args, {
        ...input.toolCtx,
        confirmed: false,
      });

      // Refus RBAC : réponse immédiate, sans laisser le modèle inventer un contournement.
      if (
        !result.ok &&
        "code" in result &&
        isBrainPermissionDenied(result.code) &&
        !("needsConfirmation" in result) &&
        !("needsChoices" in result) &&
        !("needsFileUpload" in result)
      ) {
        return materializeToolTurn(
          conversationState,
          result,
          ctas,
          { domainId: knowledge.domain.id, file: knowledge.domain.file },
          clientActions,
        );
      }

      if (!result.ok && "needsChoices" in result && result.needsChoices) {
        return materializeToolTurn(
          conversationState,
          result,
          ctas,
          { domainId: knowledge.domain.id, file: knowledge.domain.file },
          clientActions,
        );
      }

      if (!result.ok && "needsFileUpload" in result && result.needsFileUpload) {
        return materializeToolTurn(
          conversationState,
          result,
          ctas,
          { domainId: knowledge.domain.id, file: knowledge.domain.file },
          clientActions,
        );
      }

      if (!result.ok && "needsConfirmation" in result && result.needsConfirmation) {
        return materializeToolTurn(
          conversationState,
          result,
          ctas,
          { domainId: knowledge.domain.id, file: knowledge.domain.file },
          clientActions,
        );
      }

      if (result.ok) {
        // Listes PAP / ouverture pièce : réponse structurée (pas de liens inventés par le LLM).
        // materializeToolTurn ré-extrait déjà ctas/actions depuis result → ne pas pré-pousser.
        if (shouldMaterializeStructuredList(name, result)) {
          return materializeToolTurn(
            conversationState,
            result,
            ctas,
            { domainId: knowledge.domain.id, file: knowledge.domain.file },
            clientActions,
          );
        }

        ctas.push(...extractCtas(result.data));
        clientActions.push(...extractClientActions(result.data));
        const follow =
          result.data && typeof result.data === "object" && "followUrl" in (result.data as object)
            ? String((result.data as { followUrl?: string }).followUrl || "")
            : "";
        if (follow && !ctas.some((c) => c.href === follow)) {
          ctas.push({ label: "Ouvrir", href: follow });
        }
      }

      // Ne pas exposer les href PDF au LLM (sinon il les recolle en markdown cassé).
      const toolPayloadForLlm = (() => {
        if (!result.ok || !result.data || typeof result.data !== "object") return result;
        const data = { ...(result.data as Record<string, unknown>) };
        if (Array.isArray(data.eleves)) {
          data.eleves = data.eleves.map((row) => {
            if (!row || typeof row !== "object") return row;
            const e = { ...(row as Record<string, unknown>) };
            delete e.documents;
            delete e.fileHref;
            return e;
          });
        }
        delete data.fileHref;
        // Catalogue UI : résumé compact pour le LLM (les boutons sont côté chat).
        if (data.docCatalog && typeof data.docCatalog === "object") {
          const catalog = data.docCatalog as BrainDocCatalog;
          data.docCatalog = {
            title: catalog.title,
            kindLabel: catalog.kindLabel,
            total: catalog.total,
            groups: (catalog.groups || []).map((g) => ({
              title: g.title,
              count: g.count,
            })),
          };
        }
        if (Array.isArray(data.ctas)) {
          data.ctas = data.ctas.map((c) => {
            if (!c || typeof c !== "object") return c;
            const row = c as BrainCta;
            if (!row.preview) return { label: row.label, href: row.href };
            return { label: row.label, preview: true };
          });
        }
        return { ...result, data };
      })();

      messages.push({
        role: "tool",
        tool_call_id: call.id,
        name,
        content: JSON.stringify(toolPayloadForLlm),
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
    clientActions: clientActions.length ? clientActions : undefined,
  };
}
