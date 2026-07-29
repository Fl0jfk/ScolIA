import { normalizeAbsencePeriodInput } from "@/app/lib/absence-period";
import {
  computeStartEndAt,
  resolveSelfDeclarationScope,
  type AbsenceRecord,
  type AbsenceScope,
  type Etablissement,
} from "@/app/lib/absences-types";
import {
  getAbsenceIndex,
  purgeExpiredAbsences,
  saveAbsenceIndex,
  saveOrMergeAbsenceRecord,
} from "@/app/lib/absences-storage";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

export async function handleCreateAbsence(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise pour déclarer une absence.", code: "AUTH_REQUIRED" };
  }

  const scopeHint = typeof args.scope === "string" ? args.scope : undefined;
  const scope: AbsenceScope = resolveSelfDeclarationScope(ctx.roles, scopeHint);
  const etablissementRaw = typeof args.etablissement === "string" ? args.etablissement : null;
  const etablissement = (
    ["École", "Collège", "Lycée"].includes(String(etablissementRaw))
      ? etablissementRaw
      : null
  ) as Etablissement | null;

  const startDate = String(args.date || args.startDate || "").trim();
  const endDate = String(args.endDate || startDate || "").trim();
  const periodType =
    String(args.periodType || "").trim() === "multi_day" || (endDate && endDate !== startDate)
      ? "multi_day"
      : "single_day";
  const reason = String(args.reason || "").trim();
  const details = String(args.details || "").trim();
  const startTime = typeof args.startTime === "string" ? args.startTime : undefined;
  const endTime = typeof args.endTime === "string" ? args.endTime : undefined;

  if (!startDate || !/^\d{4}-\d{2}-\d{2}$/.test(startDate)) {
    return { ok: false, error: "Une date d'absence (YYYY-MM-DD) est obligatoire." };
  }
  if (!reason) {
    return { ok: false, error: "Le motif (reason) est obligatoire." };
  }
  if (scope === "professeur" && !etablissement) {
    return {
      ok: false,
      error: "Précisez l'établissement (École, Collège ou Lycée) pour une absence professeur.",
    };
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_absence",
      args: {
        scope,
        etablissement,
        periodType,
        startDate,
        endDate,
        startTime,
        endTime,
        reason,
        details,
      },
      summaryFr:
        `Déclarer mon absence le ${startDate}` +
        (endDate !== startDate ? ` → ${endDate}` : "") +
        ` — motif : ${reason} ?`,
    };
  }

  const periodResult = normalizeAbsencePeriodInput({
    periodType,
    startDate,
    endDate,
    startTime,
    endTime,
  });
  if (periodResult.error || !periodResult.data) {
    return { ok: false, error: periodResult.error || "Période invalide." };
  }
  const period = periodResult.data;
  const now = new Date().toISOString();
  const id = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  const creatorName = [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") || "Utilisateur";
  const { startAt, endAt } = computeStartEndAt({
    periodType: period.periodType,
    startDate: period.startDate,
    endDate: period.endDate,
    startTime: period.startTime,
    endTime: period.endTime,
  });

  const record: AbsenceRecord = {
    id,
    createdAt: now,
    updatedAt: now,
    source: "self",
    displayName: creatorName,
    calendarVisible: false,
    createdBy: {
      userId: ctx.userId,
      name: creatorName,
      email: ctx.email || "",
      roles: ctx.roles,
    },
    data: {
      scope,
      etablissement: scope === "ogec" ? null : etablissement,
      periodType: period.periodType,
      startDate: period.startDate,
      endDate: period.endDate,
      startTime: period.startTime ?? null,
      endTime: period.endTime ?? null,
      startAt,
      endAt,
      reason,
      details,
    },
    workflowStatus: "OUVERTE",
    managerDecision: "EN_ATTENTE",
    closedAt: null,
    justificatifRelanceAt: null,
    justification: null,
    history: [
      {
        at: now,
        by: creatorName,
        action: "CREATION",
        note: "Déclaration d'absence créée via Nico (assistant IA)",
      },
    ],
  };

  const index = await purgeExpiredAbsences(await getAbsenceIndex());
  const { index: nextIndex, record: saved, merged } = await saveOrMergeAbsenceRecord(
    index,
    record,
    creatorName,
  );
  await saveAbsenceIndex(nextIndex);

  return {
    ok: true,
    data: {
      id: saved.id,
      merged: Boolean(merged),
      followUrl: "/rh?tab=absences",
    },
    summaryFr: merged
      ? `Absence fusionnée avec une déclaration existante (${saved.id}).`
      : `Absence déclarée (${saved.id}). Vous pourrez ajouter un justificatif dans RH.`,
  };
}
