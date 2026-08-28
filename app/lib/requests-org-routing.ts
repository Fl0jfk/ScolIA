import type {
  RequestsOrgConfig,
  RequestsRoutingConfig,
  RequestServiceUnit,
  RoutingTask,
} from "@/app/lib/app-config-schemas";
import { normalizeRequestEmail } from "@/app/lib/requests-board";
import type { RequestEleveContext } from "@/app/lib/requests-eleve-context";
import type { ResolvedRequestRouting } from "@/app/lib/requests";
import { getDescendantUnitIds, getActiveUnits } from "@/app/lib/requests-org-shared";

const ROUTING_CONFIDENCE_MIN = 0.55;

export type UnitRoutingPick = {
  unitId: string;
  taskId: string;
  confidence: number;
  reason: string;
  directionHint?: string;
};

export type UnitCatalogEntry = {
  unitId: string;
  unitLabel: string;
  parentUnitId: string | null;
  parentUnitLabel: string | null;
  taskIds: string[];
  tasks: Array<{ id: string; label: string; hint: string; keywords: string[] }>;
  aggregatedKeywords: string[];
};

export function orgUnitRoutingEnabled(org: RequestsOrgConfig): boolean {
  return getActiveUnits(org).some((u) => u.taskIds.length > 0);
}

export function buildUnitCatalog(org: RequestsOrgConfig, routing: RequestsRoutingConfig): UnitCatalogEntry[] {
  const taskById = new Map(routing.tasks.map((t) => [t.id, t]));
  const unitById = new Map(getActiveUnits(org).map((u) => [u.id, u]));

  return getActiveUnits(org)
    .filter((u) => u.taskIds.length > 0)
    .map((unit) => {
      const tasks = unit.taskIds
        .map((id) => taskById.get(id))
        .filter((t): t is RoutingTask => Boolean(t && t.active));
      const keywords = new Set<string>();
      for (const t of tasks) {
        for (const k of t.keywords) keywords.add(k);
        if (t.hint) keywords.add(t.hint);
      }
      const parent = unit.parentUnitId ? unitById.get(unit.parentUnitId) : undefined;
      return {
        unitId: unit.id,
        unitLabel: unit.label,
        parentUnitId: unit.parentUnitId,
        parentUnitLabel: parent?.label ?? null,
        taskIds: tasks.map((t) => t.id),
        tasks: tasks.map((t) => ({
          id: t.id,
          label: t.label,
          hint: t.hint,
          keywords: t.keywords,
        })),
        aggregatedKeywords: [...keywords],
      };
    });
}

/** Unité responsable pour une file : parent racine plutôt que sous-service. */
export function selectPrimaryUnitForTask(org: RequestsOrgConfig, taskId: string): RequestServiceUnit | null {
  const matches = getActiveUnits(org).filter((u) => u.taskIds.includes(taskId));
  if (matches.length === 0) return null;
  if (matches.length === 1) return matches[0]!;

  const matchIds = new Set(matches.map((u) => u.id));
  const roots = matches.filter((u) => !u.parentUnitId || !matchIds.has(u.parentUnitId));
  if (roots.length === 1) return roots[0]!;
  if (roots.length > 0) {
    return roots.find((u) => u.managerEmails.length > 0) ?? roots[0]!;
  }
  return matches.find((u) => u.managerEmails.length > 0) ?? matches[0]!;
}

export function collectManagerPoolEmails(org: RequestsOrgConfig, unit: RequestServiceUnit): string[] {
  return [...new Set(unit.managerEmails.map(normalizeRequestEmail).filter(Boolean))];
}

export function collectUnitPoolEmails(
  org: RequestsOrgConfig,
  unit: RequestServiceUnit,
  taskId: string,
  includeChildUnits = true,
): string[] {
  const emails = new Set<string>();
  const add = (list: string[]) => {
    for (const e of list) {
      const n = normalizeRequestEmail(e);
      if (n) emails.add(n);
    }
  };

  add(unit.managerEmails);
  add(unit.memberEmails);

  if (includeChildUnits && unit.canDelegateToChildUnits) {
    for (const childId of getDescendantUnitIds(org, unit.id)) {
      const child = getActiveUnits(org).find((u) => u.id === childId);
      if (!child) continue;
      if (child.taskIds.length > 0 && !child.taskIds.includes(taskId)) continue;
      add(child.managerEmails);
      add(child.memberEmails);
    }
  }

  return [...emails];
}

function getActiveAssignments(routing: RequestsRoutingConfig) {
  const activeTaskIds = new Set(routing.tasks.filter((t) => t.active).map((t) => t.id));
  return routing.assignments.filter((a) => a.active && activeTaskIds.has(a.taskId));
}

function fallbackAssigneeFromRouting(routing: RequestsRoutingConfig, taskId: string): string {
  const forTask = getActiveAssignments(routing).filter((a) => a.taskId === taskId);
  if (forTask.length > 0) return forTask[0]!.email;
  const corbeille = getActiveAssignments(routing).find((a) => a.taskId === "corbeille");
  return corbeille?.email ?? "";
}

export function resolveRoutingFromUnitPick(
  routing: RequestsRoutingConfig,
  org: RequestsOrgConfig,
  pick: UnitRoutingPick,
  source: "ai" | "fallback",
): ResolvedRequestRouting {
  const taskById = new Map(routing.tasks.map((t) => [t.id, t]));
  const serviceById = new Map(routing.services.map((s) => [s.id, s]));

  let taskId = pick.taskId.trim();
  let unit =
    getActiveUnits(org).find((u) => u.id === pick.unitId) ?? selectPrimaryUnitForTask(org, taskId);

  let confidence = pick.confidence;
  let reason = pick.reason;
  const suggestedRouteId = pick.directionHint;

  // Corbeille globale uniquement si explicitement demandée ou service introuvable.
  if (taskId !== "corbeille" && !unit) {
    taskId = "corbeille";
    unit = selectPrimaryUnitForTask(org, "corbeille");
    reason = `Aucun service configuré pour cette file — corbeille. ${reason}`.trim();
    confidence = Math.min(confidence, 0.35);
  }

  if (!unit) {
    unit = selectPrimaryUnitForTask(org, taskId);
  }

  // Service identifié mais personne incertaine → pile du service (managers), pas une personne.
  if (taskId !== "corbeille" && unit && confidence < ROUTING_CONFIDENCE_MIN) {
    reason = `Pile ${unit.label} — service identifié, personne non ciblée (confiance ${Math.round(confidence * 100)}%). ${reason}`.trim();
  }

  const task = taskById.get(taskId);
  const assignmentForService = getActiveAssignments(routing).find((a) => a.taskId === taskId);
  const service = assignmentForService
    ? serviceById.get(assignmentForService.serviceId)
    : routing.services[0];

  const managerPool = unit ? collectManagerPoolEmails(org, unit) : [];
  const fullPool = unit ? collectUnitPoolEmails(org, unit, taskId, false) : [];
  const poolEmails =
    managerPool.length > 0
      ? managerPool
      : fullPool.length > 0
        ? fullPool
        : getActiveAssignments(routing)
            .filter((a) => a.taskId === taskId)
            .map((a) => normalizeRequestEmail(a.email))
            .filter(Boolean);

  const primaryEmail = poolEmails[0] || fallbackAssigneeFromRouting(routing, taskId);

  const unitLabel = unit?.label ?? "Service";
  const taskLabel = task?.label ?? taskId;

  return {
    category: service?.category || "Général",
    assignedTo: {
      routeId: taskId,
      unit: taskId,
      roleLabel: `${taskLabel} — ${unitLabel}`,
      email: primaryEmail,
      claimedBy: null,
      ...(poolEmails.length > 0 ? { poolEmails } : {}),
    },
    source,
    confidence,
    reason,
    ...(suggestedRouteId ? { suggestedRouteId } : {}),
    routingMeta: {
      assignmentId: assignmentForService?.id ?? "",
      taskId,
      unitId: unit?.id ?? pick.unitId,
      servicePile: taskId !== "corbeille",
    },
  };
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

type UnitScore = {
  unitId: string;
  taskId: string;
  score: number;
  matchedKeywords: string[];
};

function secteurBoostForUnit(unit: RequestServiceUnit, secteur: string | null | undefined): number {
  if (!secteur) return 0;
  const hay = normalizeMatchText(`${unit.id} ${unit.label}`);
  if (secteur === "lycee" && (hay.includes("lycee") || hay.includes("lycée"))) return 4;
  if (secteur === "college" && hay.includes("college")) return 4;
  if (secteur === "ecole" && (hay.includes("ecole") || hay.includes("école"))) return 4;
  return 0;
}

/** Fallback local : score par unité + file (mots-clés des tâches rattachées). */
export function keywordFallbackByUnit(
  org: RequestsOrgConfig,
  routing: RequestsRoutingConfig,
  subject: string,
  description: string,
  eleveCtx?: RequestEleveContext | null,
): UnitRoutingPick | null {
  const catalog = buildUnitCatalog(org, routing);
  if (catalog.length === 0) return null;

  const textNorm = normalizeMatchText(`${subject} ${description}`);
  const scores: UnitScore[] = [];

  const suggestedSecteur =
    eleveCtx?.suggestedSecteur ??
    (eleveCtx?.textSecteurHints.length === 1 ? eleveCtx.textSecteurHints[0] : null);

  for (const entry of catalog) {
    const unit = getActiveUnits(org).find((u) => u.id === entry.unitId);
    const secteurBonus = unit ? secteurBoostForUnit(unit, suggestedSecteur) : 0;

    for (const task of entry.tasks) {
      let score = secteurBonus;
      const matched: string[] = [];
      if (secteurBonus > 0 && suggestedSecteur) matched.push(`cycle ${suggestedSecteur}`);
      for (const kw of task.keywords) {
        const k = normalizeMatchText(kw);
        if (k.length >= 2 && textNorm.includes(k)) {
          score += 2;
          matched.push(kw);
        }
      }
      const hint = normalizeMatchText(task.hint);
      if (hint && textNorm.includes(hint)) {
        score += 1;
        matched.push(task.hint);
      }
      if (score > 0) {
        scores.push({ unitId: entry.unitId, taskId: task.id, score, matchedKeywords: matched });
      }
    }
  }

  if (scores.length === 0) return null;

  scores.sort((a, b) => b.score - a.score);
  const best = scores[0]!;
  const second = scores[1];
  if (second && second.score >= best.score * 0.85 && best.unitId !== second.unitId) {
    const corbeilleUnit = selectPrimaryUnitForTask(org, "corbeille");
    return {
      unitId: corbeilleUnit?.id ?? best.unitId,
      taskId: "corbeille",
      confidence: 0.38,
      reason: `Ambiguïté entre services (scores ${best.score} vs ${second.score}) — corbeille établissement.`,
    };
  }

  const primaryUnit = selectPrimaryUnitForTask(org, best.taskId);
  const conf = Math.min(0.88, 0.48 + best.score * 0.08);
  return {
    unitId: primaryUnit?.id ?? best.unitId,
    taskId: best.taskId,
    confidence: conf,
    reason:
      conf < ROUTING_CONFIDENCE_MIN
        ? `Pile ${primaryUnit?.label ?? "service"} — mots-clés partiels (${best.matchedKeywords.join(", ")}).`
        : `Service identifié via mots-clés (${best.matchedKeywords.join(", ")}).`,
  };
}

export function corbeilleUnitFallback(
  routing: RequestsRoutingConfig,
  org: RequestsOrgConfig,
  reason: string,
  source: "ai" | "fallback",
): ResolvedRequestRouting {
  const unit = selectPrimaryUnitForTask(org, "corbeille");
  return resolveRoutingFromUnitPick(
    routing,
    org,
    {
      unitId: unit?.id ?? "corbeille",
      taskId: "corbeille",
      confidence: 0.32,
      reason,
    },
    source,
  );
}

export function assignmentPickToUnitPick(
  org: RequestsOrgConfig,
  routing: RequestsRoutingConfig,
  assignmentId: string,
  confidence: number,
  reason: string,
  directionHint?: string,
): UnitRoutingPick | null {
  const assignment = routing.assignments.find((a) => a.id === assignmentId);
  if (!assignment) return null;
  const unit = selectPrimaryUnitForTask(org, assignment.taskId);
  if (!unit) return null;
  return {
    unitId: unit.id,
    taskId: assignment.taskId,
    confidence,
    reason,
    directionHint,
  };
}
