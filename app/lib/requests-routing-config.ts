import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  parseRequestsRouting,
  type RequestsRoutingConfig,
  type RoutingAssignment,
  type RoutingTask,
} from "@/app/lib/app-config-schemas";
import { loadAppConfig } from "@/app/lib/app-config";
import { defaultEstablishments } from "@/app/lib/app-config-defaults";
import { parseEstablishmentsFile, type Establishment } from "@/app/lib/app-config-schemas";
import {
  countSitesInDb,
  isEntCoreDbEnabled,
  listSitesFromDb,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import { defaultRequestsRouting, RH_REQUEST_ROUTE_ID, isManualOnlyDirectionRoute, syncDirectionQueuesFromTasks, syncRequestsRoutingWithEstablishments } from "@/app/lib/requests-routing-defaults";
import { syncStaffDirectoryFromRequestsConfig, getRequestsOrgConfig } from "@/app/lib/requests-org-config";
import {
  buildUnitCatalog,
  corbeilleUnitFallback,
  keywordFallbackByUnit,
  orgUnitRoutingEnabled,
  resolveRoutingFromUnitPick,
  type UnitRoutingPick,
} from "@/app/lib/requests-org-routing";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import type { ResolvedRequestRouting } from "@/app/lib/requests";
import {
  buildRequestEleveContext,
  tagsMatchSecteur,
  type RequestEleveContext,
} from "@/app/lib/requests-eleve-context";

const ROUTING_KEY = "settings/requests-routing.json";
const CACHE_MS = 45_000;
/** Sous ce seuil → corbeille globale (doute). */
const ROUTING_CONFIDENCE_MIN = 0.55;
/** Score tag minimum pour sortir de la corbeille (fallback local). */
const TAG_MATCH_MIN_SCORE = 3;
/** Si le 2e score est trop proche du 1er → ambiguïté → corbeille. */
const AMBIGUITY_RATIO = 0.85;
const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";

let cache: { at: number; config: RequestsRoutingConfig } | null = null;

async function loadEstablishmentsForRouting(): Promise<Establishment[]> {
  try {
    const estRaw = await getJson<unknown>("settings/establishments.json");
    let establishments = estRaw?.data
      ? parseEstablishmentsFile(estRaw.data)
      : defaultEstablishments();
    if (isEntCoreDbEnabled()) {
      try {
        const etabId = await resolveCurrentEtablissementId();
        if (etabId && (await countSitesInDb(etabId)) > 0) {
          establishments = await listSitesFromDb(etabId);
        }
      } catch {
        /* ignore */
      }
    }
    return establishments;
  } catch {
    return defaultEstablishments();
  }
}

function invalidateRequestsRoutingCache() {
  cache = null;
}

/** Assure la présence de la file RH même sur une config tenant plus ancienne. */
function ensureBuiltinRhRouting(config: RequestsRoutingConfig): RequestsRoutingConfig {
  const defaults = defaultRequestsRouting();
  const rhService = defaults.services.find((s) => s.id === "rh");
  const rhTask = defaults.tasks.find((t) => t.id === RH_REQUEST_ROUTE_ID);
  let services = config.services;
  let tasks = config.tasks;
  let changed = false;

  if (rhService && !services.some((s) => s.id === "rh")) {
    services = [...services, rhService];
    changed = true;
  }
  if (rhTask && !tasks.some((t) => t.id === RH_REQUEST_ROUTE_ID)) {
    tasks = [...tasks, rhTask];
    changed = true;
  }
  if (!changed) return config;
  return { ...config, services, tasks };
}

export async function getRequestsRoutingConfig(): Promise<RequestsRoutingConfig> {
  if (cache && Date.now() - cache.at < CACHE_MS) return cache.config;
  const raw = await getJson<{ data?: unknown }>(ROUTING_KEY);
  let config = ensureBuiltinRhRouting(
    raw?.data ? parseRequestsRouting(raw.data) : defaultRequestsRouting(),
  );
  const needsDirectionSync =
    config.directionQueues.length === 0 ||
    config.directionQueues.every((q) => !q.email?.trim());
  if (needsDirectionSync) {
    try {
      const establishments = await loadEstablishmentsForRouting();
      config = syncDirectionQueuesFromTasks(
        syncRequestsRoutingWithEstablishments(config, establishments),
        establishments,
      );
    } catch {
      config = syncDirectionQueuesFromTasks(config);
    }
  }
  cache = { at: Date.now(), config };
  return config;
}

export type SaveRequestsRoutingOptions = {
  /** Conserve tagCatalog / personnelTags existants si le payload les vide (sauvegarde structurelle). */
  preserveTags?: boolean;
};

function mergeRoutingTagsFromExisting(
  incoming: RequestsRoutingConfig,
  existing: RequestsRoutingConfig,
): RequestsRoutingConfig {
  const inTags = incoming.personnelTags ?? [];
  const inCatalog = incoming.tagCatalog ?? [];
  const exTags = existing.personnelTags ?? [];
  const exCatalog = existing.tagCatalog ?? [];
  const wouldWipe =
    inTags.length === 0 && inCatalog.length === 0 && (exTags.length > 0 || exCatalog.length > 0);
  if (!wouldWipe) return incoming;
  return {
    ...incoming,
    personnelTags: exTags,
    tagCatalog: exCatalog,
  };
}

export async function saveRequestsRoutingConfig(
  config: RequestsRoutingConfig,
  options?: SaveRequestsRoutingOptions,
): Promise<RequestsRoutingConfig> {
  invalidateRequestsRoutingCache();
  const existing = await getRequestsRoutingConfig();
  const merged =
    options?.preserveTags === false ? config : mergeRoutingTagsFromExisting(config, existing);
  const parsed = parseRequestsRouting(merged);
  let enriched = parsed;
  try {
    const establishments = await loadEstablishmentsForRouting();
    enriched = syncDirectionQueuesFromTasks(
      syncRequestsRoutingWithEstablishments(parsed, establishments),
      establishments,
    );
  } catch {
    enriched = syncDirectionQueuesFromTasks(parsed);
  }
  await putJson(ROUTING_KEY, { version: 1, updatedAt: new Date().toISOString(), data: enriched });
  await syncStaffDirectoryFromRequestsConfig(enriched);
  invalidateRequestsRoutingCache();
  return enriched;
}

function getActiveTasks(config: RequestsRoutingConfig): RoutingTask[] {
  return config.tasks.filter((t) => t.active);
}

function getActiveAssignments(config: RequestsRoutingConfig): RoutingAssignment[] {
  const activeTaskIds = new Set(getActiveTasks(config).map((t) => t.id));
  return config.assignments.filter((a) => a.active && activeTaskIds.has(a.taskId));
}

function getAllStaffEmailsFromRouting(config: RequestsRoutingConfig): string[] {
  const emails = new Set<string>();
  for (const a of getActiveAssignments(config)) emails.add(a.email.toLowerCase());
  for (const d of config.directionQueues.filter((q) => q.active)) emails.add(d.email.toLowerCase());
  return [...emails];
}

export function isListedInRouting(config: RequestsRoutingConfig, email: string): boolean {
  const e = email.trim().toLowerCase();
  return getAllStaffEmailsFromRouting(config).includes(e);
}

function getAssignmentsForEmail(config: RequestsRoutingConfig, email: string): RoutingAssignment[] {
  const e = email.trim().toLowerCase();
  return getActiveAssignments(config).filter((a) => a.email.toLowerCase() === e);
}

type AiRoutingPick = {
  assignmentId: string;
  confidence: number;
  reason: string;
  directionHint?: string;
};

function buildCatalogPayload(config: RequestsRoutingConfig) {
  const activeAssignments = getActiveAssignments(config);
  const taskById = new Map(config.tasks.map((t) => [t.id, t]));
  const serviceById = new Map(config.services.map((s) => [s.id, s]));
  const tagsByEmail = new Map(
    (config.personnelTags || []).map((p) => [p.email.toLowerCase(), p.tags]),
  );

  /** Tags agrégés par service (union des tags des personnes du service). */
  const serviceTagsById = new Map<string, string[]>();
  for (const a of activeAssignments) {
    const personTags = tagsByEmail.get(a.email.toLowerCase()) || [];
    if (personTags.length === 0) continue;
    const prev = serviceTagsById.get(a.serviceId) || [];
    serviceTagsById.set(a.serviceId, [...new Set([...prev, ...personTags])]);
  }

  const catalog = activeAssignments.map((a) => {
    const task = taskById.get(a.taskId);
    const service = serviceById.get(a.serviceId);
    const personTags = tagsByEmail.get(a.email.toLowerCase()) || [];
    return {
      assignmentId: a.id,
      taskId: a.taskId,
      taskLabel: task?.label || a.taskId,
      taskHint: task?.hint || "",
      taskKeywords: task?.keywords || [],
      personName: a.personName,
      email: a.email,
      personTags,
      serviceId: a.serviceId,
      serviceLabel: service?.label || a.serviceId,
      serviceCategory: service?.category || "",
      serviceTags: serviceTagsById.get(a.serviceId) || [],
      isGlobalInbox: a.taskId === "corbeille",
    };
  });

  const directionHints = config.directionQueues
    .filter((q) => q.active)
    .map((q) => ({ id: q.id, label: q.label, email: q.email }));

  const corbeilleAssignmentId =
    activeAssignments.find((a) => a.taskId === "corbeille")?.id || null;

  return { catalog, directionHints, corbeilleAssignmentId };
}

function normalizeMatchText(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^\p{L}\p{N}\s]/gu, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Compte combien de tags/mots-clés apparaissent clairement dans le texte. */
function countTagHits(textNorm: string, tags: string[]): { hits: number; matched: string[] } {
  const matched: string[] = [];
  for (const raw of tags) {
    const tag = normalizeMatchText(raw);
    if (!tag || tag.length < 2) continue;
    // Mot entier ou expression multi-mots
    const escaped = tag.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const re = new RegExp(`(?:^|\\s)${escaped}(?:\\s|$)`);
    if (re.test(` ${textNorm} `) || textNorm.includes(tag)) {
      matched.push(raw);
    }
  }
  return { hits: matched.length, matched };
}

type MatchScore = {
  assignment: RoutingAssignment;
  score: number;
  personHits: number;
  taskHits: number;
  secteurHits: number;
  matchedPersonTags: string[];
  matchedTaskKeywords: string[];
  matchedSecteurTags: string[];
};

function scoreAssignments(
  config: RequestsRoutingConfig,
  subject: string,
  description: string,
  eleveCtx?: RequestEleveContext | null,
): MatchScore[] {
  const textNorm = normalizeMatchText(`${subject} ${description}`);
  const activeAssignments = getActiveAssignments(config);
  const taskById = new Map(config.tasks.map((t) => [t.id, t]));
  const tagsByEmail = new Map(
    (config.personnelTags || []).map((p) => [p.email.toLowerCase(), p.tags]),
  );

  const scored: MatchScore[] = [];
  for (const a of activeAssignments) {
    if (a.taskId === "corbeille") continue;
    const task = taskById.get(a.taskId);
    if (!task) continue;
    const personTags = tagsByEmail.get(a.email.toLowerCase()) || [];
    const person = countTagHits(textNorm, personTags);
    const taskKw = countTagHits(textNorm, task.keywords || []);

    let secteurHits = 0;
    const matchedSecteurTags: string[] = [];
    if (eleveCtx?.suggestedSecteur) {
      const secteurMatched = tagsMatchSecteur(personTags, eleveCtx.suggestedSecteur);
      secteurHits = secteurMatched.length;
      matchedSecteurTags.push(...secteurMatched);
    }

    // Tags personne + cycle (élève) + mots-clés tâche
    const score = person.hits * 4 + secteurHits * 5 + taskKw.hits * 2;
    if (score <= 0) continue;
    scored.push({
      assignment: a,
      score,
      personHits: person.hits,
      taskHits: taskKw.hits,
      secteurHits,
      matchedPersonTags: person.matched,
      matchedTaskKeywords: taskKw.matched,
      matchedSecteurTags,
    });
  }

  scored.sort((a, b) => {
    if (b.score !== a.score) return b.score - a.score;
    if (b.secteurHits !== a.secteurHits) return b.secteurHits - a.secteurHits;
    if (b.personHits !== a.personHits) return b.personHits - a.personHits;
    return b.taskHits - a.taskHits;
  });
  return scored;
}

/**
 * Matching local : maximise les tags → personne ; sinon service clair ;
 * cycle élève (eleves.json) pour départager lycée/collège/école ;
 * doute / égalité → corbeille globale.
 */
function keywordFallback(
  config: RequestsRoutingConfig,
  subject: string,
  description: string,
  eleveCtx?: RequestEleveContext | null,
): AiRoutingPick | null {
  const corbeille = getActiveAssignments(config).find((a) => a.taskId === "corbeille");
  const toCorbeille = (reason: string, confidence = 0.32): AiRoutingPick | null => {
    if (!corbeille) return null;
    return { assignmentId: corbeille.id, confidence, reason };
  };

  const scored = scoreAssignments(config, subject, description, eleveCtx);
  if (scored.length === 0) {
    const base = "Aucun tag / mot-clé clair — corbeille établissement.";
    return toCorbeille(eleveCtx?.summary ? `${base} ${eleveCtx.summary}` : base);
  }

  const best = scored[0]!;
  if (best.score < TAG_MATCH_MIN_SCORE) {
    return toCorbeille(
      `Signal trop faible (score ${best.score}) — corbeille. ${eleveCtx?.summary || ""}`.trim(),
    );
  }

  const second = scored[1];
  if (second && second.score >= best.score * AMBIGUITY_RATIO) {
    const sameService = second.assignment.serviceId === best.assignment.serviceId;
    const sameTask = second.assignment.taskId === best.assignment.taskId;

    // Si le cycle élève départage clairement → garder le meilleur secteur
    if (
      !sameService &&
      !sameTask &&
      best.secteurHits > 0 &&
      best.secteurHits > second.secteurHits
    ) {
      // continue with best below
    } else if (sameService || sameTask) {
      const confidence = Math.min(
        0.74,
        0.48 + best.personHits * 0.08 + best.secteurHits * 0.1 + best.taskHits * 0.04,
      );
      return {
        assignmentId: best.assignment.id,
        confidence,
        reason: `Service/tâche clair via tags${
          eleveCtx?.suggestedSecteur ? ` + cycle ${eleveCtx.suggestedSecteur}` : ""
        } (${best.personHits} tag(s), ${best.secteurHits} cycle). ${eleveCtx?.summary || ""}`.trim(),
      };
    } else {
      return toCorbeille(
        `Ambiguïté entre ${best.assignment.personName} (score ${best.score}) et ${second.assignment.personName} (score ${second.score}) — corbeille globale. ${eleveCtx?.summary || ""}`.trim(),
        0.38,
      );
    }
  }

  const confidence = Math.min(
    0.9,
    0.5 + best.personHits * 0.1 + best.secteurHits * 0.12 + best.taskHits * 0.05,
  );
  const focus =
    best.secteurHits > 0
      ? `cycle ${eleveCtx?.suggestedSecteur} → ${best.assignment.personName}`
      : best.personHits > 0
        ? `personne ${best.assignment.personName}`
        : `service / tâche ${best.assignment.taskId}`;
  return {
    assignmentId: best.assignment.id,
    confidence,
    reason: `Matching max tags → ${focus} (score ${best.score} : ${[
      ...best.matchedSecteurTags,
      ...best.matchedPersonTags,
      ...best.matchedTaskKeywords,
    ].join(", ")}). ${eleveCtx?.summary || ""}`.trim(),
  };
}

async function callMistralRouting(
  subject: string,
  description: string,
  config: RequestsRoutingConfig,
  eleveCtx?: RequestEleveContext | null,
): Promise<AiRoutingPick | null> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) return null;

  const { catalog, directionHints, corbeilleAssignmentId } = buildCatalogPayload(config);
  if (catalog.length === 0) return null;

  const system = `Tu es le routeur de demandes internes d'un groupe scolaire (école / collège / lycée possibles).
Catalogue JSON : chaque entrée = une tâche + une personne.
- personTags : compétences ET rattachement d'établissement (ex. "lycée", "collège", "école", "secrétariat lycée", plomberie…)
- serviceTags : union des tags du service
- taskKeywords : mots-clés de la tâche
- isGlobalInbox=true : corbeille établissement (file globale)

Contexte élèves (eleves.json) fourni à part :
- hits : élèves / e-mails parents reconnus dans la demande, avec secteur (ecole|college|lycee)
- suggestedSecteur : cycle à privilégier si cohérent

Règles de décision STRICTES (dans l'ordre) :
1. Si suggestedSecteur est connu, privilégie fortement les personnes dont les personTags mentionnent ce cycle (lycée / collège / école) — ex. secrétariat lycée vs secrétariat collège.
2. Maximiser ensuite les personTags métier qui matchent le texte → choisis CETTE personne.
3. Si aucune personne ne se détache mais un service/cycle est clair → affectation de ce service.
4. Si plusieurs personnes/services DIFFÉRENTS restent crédibles → DOUTE : corbeille globale (assignmentId=${corbeilleAssignmentId || "corbeille"}), confidence <= 0.4.
5. Si aucun tag / cycle clair → corbeille globale, confidence <= 0.35.
6. Ne choisis une personne hors corbeille que si confidence >= 0.55.

Les files direction (direction_ecole, direction_college, direction_lycee) ne sont PAS dans le catalogue : si la demande concerne clairement la direction, renvoie directionHint avec l'id approprié mais choisis une affectation non-direction pour le traitement initial.
Réponds UNIQUEMENT en JSON valide : {"assignmentId":"...","confidence":0.0-1.0,"reason":"...","directionHint":null ou "direction_..."}`;

  const appConfig = await loadAppConfig();
  const establishmentLabels = Object.fromEntries(
    appConfig.establishments.map((e) => [e.id, e.label]),
  );

  const user = JSON.stringify({
    subject,
    description,
    catalog,
    directionHints,
    corbeilleAssignmentId,
    eleveContext: eleveCtx
      ? {
          hits: eleveCtx.hits,
          suggestedSecteur: eleveCtx.suggestedSecteur,
          textSecteurHints: eleveCtx.textSecteurHints,
          summary: eleveCtx.summary,
        }
      : null,
    rule: "max_tags_then_person_or_service_with_eleve_secteur_else_global_inbox",
    establishments: establishmentLabels,
  });

  const res = await fetch(MISTRAL_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      temperature: 0.1,
      response_format: { type: "json_object" },
      messages: [
        { role: "system", content: system },
        { role: "user", content: user },
      ],
    }),
  });

  if (!res.ok) return null;
  const data = (await res.json()) as {
    choices?: { message?: { content?: string } }[];
  };
  const content = data.choices?.[0]?.message?.content;
  if (!content) return null;

  try {
    const parsed = JSON.parse(content) as AiRoutingPick;
    if (!parsed.assignmentId) return null;
    const exists = catalog.some((c) => c.assignmentId === parsed.assignmentId);
    if (!exists) return null;
    return {
      assignmentId: parsed.assignmentId,
      confidence: typeof parsed.confidence === "number" ? parsed.confidence : 0.5,
      reason: parsed.reason || "Routage IA",
      directionHint: parsed.directionHint || undefined,
    };
  } catch {
    return null;
  }
}

function pickToResolved(
  config: RequestsRoutingConfig,
  pick: AiRoutingPick,
  source: "ai" | "fallback",
): ResolvedRequestRouting {
  const assignmentById = new Map(config.assignments.map((a) => [a.id, a]));
  const taskById = new Map(config.tasks.map((t) => [t.id, t]));
  const serviceById = new Map(config.services.map((s) => [s.id, s]));

  const assignment = assignmentById.get(pick.assignmentId);
  if (!assignment) {
    return corbeilleFallback(config, "Affectation introuvable.", source);
  }

  const task = taskById.get(assignment.taskId);
  const service = serviceById.get(assignment.serviceId);
  const poolEmails = [
    ...new Set(
      getActiveAssignments(config)
        .filter((a) => a.taskId === assignment.taskId)
        .map((a) => a.email),
    ),
  ];

  let confidence = pick.confidence;
  let reason = pick.reason;
  let suggestedRouteId = pick.directionHint;
  let chosenAssignment = assignment;

  const corbeilleAssignment = getActiveAssignments(config).find((a) => a.taskId === "corbeille");

  if (
    confidence < ROUTING_CONFIDENCE_MIN &&
    assignment.taskId !== "corbeille" &&
    corbeilleAssignment
  ) {
    suggestedRouteId = suggestedRouteId || assignment.taskId;
    chosenAssignment = corbeilleAssignment;
    reason = `Doute (confiance ${Math.round(confidence * 100)}% < ${Math.round(ROUTING_CONFIDENCE_MIN * 100)}%) → corbeille globale. Hypothèse : ${assignment.personName} / ${assignment.taskId}. ${pick.reason}`.trim();
    confidence = Math.min(confidence, 0.42);
  }

  const chosenTask = taskById.get(chosenAssignment.taskId);
  const chosenService = serviceById.get(chosenAssignment.serviceId);
  const chosenPool = [
    ...new Set(
      getActiveAssignments(config)
        .filter((a) => a.taskId === chosenAssignment.taskId)
        .map((a) => a.email),
    ),
  ];

  return {
    category: chosenService?.category || "Général",
    assignedTo: {
      routeId: chosenAssignment.taskId,
      unit: chosenAssignment.taskId,
      roleLabel: `${chosenTask?.label || chosenAssignment.taskId} — ${chosenAssignment.personName}`,
      email: chosenAssignment.email,
      claimedBy: null,
      ...(chosenPool.length > 1 ? { poolEmails: chosenPool } : {}),
    },
    source,
    confidence,
    reason,
    ...(suggestedRouteId ? { suggestedRouteId } : {}),
    routingMeta: { assignmentId: chosenAssignment.id, taskId: chosenAssignment.taskId },
  };
}

function corbeilleFallback(
  config: RequestsRoutingConfig,
  reason: string,
  source: "ai" | "fallback",
): ResolvedRequestRouting {
  const corbeille = getActiveAssignments(config).find((a) => a.taskId === "corbeille");
  const task = config.tasks.find((t) => t.id === "corbeille");
  const service = config.services.find((s) => s.id === "etablissement");
  const email = corbeille?.email || "";
  return {
    category: service?.category || "Établissement",
    assignedTo: {
      routeId: "corbeille",
      unit: "corbeille",
      roleLabel: task?.label || "Corbeille établissement",
      email,
      claimedBy: null,
    },
    source,
    confidence: 0.32,
    reason,
  };
}

type AiUnitRoutingPick = UnitRoutingPick;

function buildUnitCatalogPayload(org: Awaited<ReturnType<typeof getRequestsOrgConfig>>, config: RequestsRoutingConfig) {
  const catalog = buildUnitCatalog(org, config);
  const corbeilleUnit = catalog.find((e) => e.taskIds.includes("corbeille")) ?? null;
  return {
    catalog: catalog.map((e) => ({
      unitId: e.unitId,
      unitLabel: e.unitLabel,
      parentUnitLabel: e.parentUnitLabel,
      tasks: e.tasks,
      aggregatedKeywords: e.aggregatedKeywords,
    })),
    corbeilleUnitId: corbeilleUnit?.unitId ?? null,
  };
}

async function callMistralUnitRouting(
  subject: string,
  description: string,
  org: Awaited<ReturnType<typeof getRequestsOrgConfig>>,
  config: RequestsRoutingConfig,
  eleveCtx?: RequestEleveContext | null,
): Promise<AiUnitRoutingPick | null> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) return null;

  const { catalog, corbeilleUnitId } = buildUnitCatalogPayload(org, config);
  if (catalog.length === 0) return null;

  const system = `Tu es le routeur de demandes internes d'un groupe scolaire.
Catalogue JSON : chaque entrée = un SERVICE (unitId) avec une ou plusieurs FILES (tasks) et leurs mots-clés.
Tu ne choisis JAMAIS une personne — uniquement unitId + taskId.

Contexte élèves (eleves.json) fourni à part :
- suggestedSecteur : cycle à privilégier (ecole|college|lycee) si cohérent avec le texte

Règles STRICTES :
1. Si suggestedSecteur est connu, privilégie les unités dont le label/id évoque ce cycle (CPE lycée vs CPE collège…).
2. Si le SERVICE et la FILE sont clairs → unitId + taskId correspondants, confidence >= 0.55.
3. Si le SERVICE est identifiable mais la personne ne l'est PAS → même unitId/taskId, confidence 0.35–0.54 (pile du service, PAS corbeille).
4. Corbeille (taskId=corbeille, unitId=${corbeilleUnitId || "corbeille"}) UNIQUEMENT si :
   - ambiguïté entre PLUSIEURS services différents, OU
   - aucune correspondance crédible.
5. Ne redirige JAMAIS vers la corbeille uniquement parce que la personne est incertaine.

Réponds UNIQUEMENT en JSON valide : {"unitId":"...","taskId":"...","confidence":0.0-1.0,"reason":"...","directionHint":null ou "direction_..."}`;

  const appConfig = await loadAppConfig();
  const establishmentLabels = Object.fromEntries(
    appConfig.establishments.map((e) => [e.id, e.label]),
  );

  const user = JSON.stringify({
    subject,
    description,
    catalog,
    corbeilleUnitId,
    eleveContext: eleveCtx
      ? {
          hits: eleveCtx.hits,
          suggestedSecteur: eleveCtx.suggestedSecteur,
          textSecteurHints: eleveCtx.textSecteurHints,
          summary: eleveCtx.summary,
        }
      : null,
    rule: "service_and_task_not_person_low_confidence_to_service_pile_else_corbeille",
    establishments: establishmentLabels,
  });

  const res = await fetch(MISTRAL_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      temperature: 0.1,
      response_format: { type: "json_object" },
      messages: [
        { role: "system", content: system },
        { role: "user", content: user },
      ],
    }),
  });

  if (!res.ok) return null;
  const data = (await res.json()) as {
    choices?: { message?: { content?: string } }[];
  };
  const content = data.choices?.[0]?.message?.content;
  if (!content) return null;

  try {
    const parsed = JSON.parse(content) as AiUnitRoutingPick;
    if (!parsed.unitId || !parsed.taskId) return null;
    const validUnit = catalog.some((c) => c.unitId === parsed.unitId);
    if (!validUnit && parsed.taskId !== "corbeille") return null;
    return {
      unitId: parsed.unitId,
      taskId: parsed.taskId,
      confidence: typeof parsed.confidence === "number" ? parsed.confidence : 0.5,
      reason: parsed.reason || "Routage IA (service)",
      directionHint: parsed.directionHint || undefined,
    };
  } catch {
    return null;
  }
}

async function resolveRoutingFromOrgCatalog(
  subject: string,
  description: string,
  config: RequestsRoutingConfig,
  eleveCtx: RequestEleveContext,
): Promise<ResolvedRequestRouting> {
  const org = await getRequestsOrgConfig();

  const aiPick = await callMistralUnitRouting(subject, description, org, config, eleveCtx);
  if (aiPick) return resolveRoutingFromUnitPick(config, org, aiPick, "ai");

  const kwPick = keywordFallbackByUnit(org, config, subject, description, eleveCtx);
  if (kwPick) return resolveRoutingFromUnitPick(config, org, kwPick, "fallback");

  return corbeilleUnitFallback(
    config,
    org,
    `Aucune correspondance service — corbeille. ${eleveCtx.summary}`.trim(),
    "fallback",
  );
}

async function resolveRoutingFromLegacyCatalog(
  subject: string,
  description: string,
  config: RequestsRoutingConfig,
  eleveCtx: RequestEleveContext,
): Promise<ResolvedRequestRouting> {
  const aiPick = await callMistralRouting(subject, description, config, eleveCtx);
  if (aiPick) return pickToResolved(config, aiPick, "ai");

  const kwPick = keywordFallback(config, subject, description, eleveCtx);
  if (kwPick) return pickToResolved(config, kwPick, "fallback");

  return corbeilleFallback(
    config,
    `Catalogue vide ou sans correspondance — corbeille. ${eleveCtx.summary}`.trim(),
    "fallback",
  );
}

export async function resolveRoutingFromCatalog(
  subject: string,
  description: string,
): Promise<ResolvedRequestRouting> {
  const config = await getRequestsRoutingConfig();
  const eleveCtx = await buildRequestEleveContext(subject, description);
  const org = await getRequestsOrgConfig();

  if (orgUnitRoutingEnabled(org)) {
    return resolveRoutingFromOrgCatalog(subject, description, config, eleveCtx);
  }

  return resolveRoutingFromLegacyCatalog(subject, description, config, eleveCtx);
}

export async function getAllBranchStaffEmailsFromRouting(): Promise<string[]> {
  const config = await getRequestsRoutingConfig();
  return getAllStaffEmailsFromRouting(config);
}

export function listActiveTasksForPicker(config: RequestsRoutingConfig) {
  return getActiveTasks(config).map((t) => {
    const service = config.services.find((s) =>
      getActiveAssignments(config).some((a) => a.taskId === t.id && a.serviceId === s.id),
    );
    return {
      id: t.id,
      label: t.label,
      category: service?.category || "Général",
    };
  });
}

export function listDirectionQueuesForTransmit(config: RequestsRoutingConfig) {
  const fromQueues = config.directionQueues
    .filter((q) => q.active && isManualOnlyDirectionRoute(q.id))
    .map((q) => ({ id: q.id, label: q.label, category: "Direction" as const }));
  if (fromQueues.length > 0) return fromQueues;

  return config.tasks
    .filter((t) => t.active && isManualOnlyDirectionRoute(t.id))
    .map((t) => ({ id: t.id, label: t.label, category: "Direction" as const }));
}
