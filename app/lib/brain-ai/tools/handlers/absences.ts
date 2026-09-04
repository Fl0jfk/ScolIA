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
  saveOrMergeAbsenceRecord,
} from "@/app/lib/absences-storage";
import { loadAppConfig } from "@/app/lib/app-config";
import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import {
  buildDateQuickOptions,
  weekdayLabelFr,
  wizardStep,
  WIZARD_DATE_OTHER,
} from "@/app/lib/brain-ai/wizard";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { establishmentChoiceOptions, matchEstablishment } from "@/app/lib/establishment-catalog";

const COMMON_REASONS = [
  "Maladie",
  "Rendez-vous médical",
  "Formation",
  "Congé",
  "Convocation",
  "Autre",
];

/**
 * Wizard absence : date → durée → (fin si multi) → motif → établissement? → détails? → confirmation
 */
export async function handleCreateAbsence(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise pour demander une autorisation d'absence.", code: "AUTH_REQUIRED" };
  }

  const scopeHint = typeof args.scope === "string" ? args.scope : undefined;
  const scope: AbsenceScope = resolveSelfDeclarationScope(ctx.roles, scopeHint);
  const bundle = await loadAppConfig();
  const etabChoices = establishmentChoiceOptions(bundle.establishments);
  const etablissementRaw = typeof args.etablissement === "string" ? args.etablissement : null;
  const matchedEtab = matchEstablishment(bundle.establishments, etablissementRaw);
  let etablissement = (matchedEtab?.label || null) as Etablissement | null;

  let startDate = String(args.date || args.startDate || "").trim();
  let endDate = String(args.endDate || "").trim();
  const periodTypeRaw = String(args.periodType || "").trim();
  let periodType: "single_day" | "multi_day" | "" =
    periodTypeRaw === "multi_day"
      ? "multi_day"
      : periodTypeRaw === "single_day"
        ? "single_day"
        : "";
  let reason = String(args.reason || "").trim();
  const details = String(args.details || "").trim();
  const detailsResolved = Boolean(args.detailsResolved);
  const startTime = typeof args.startTime === "string" ? args.startTime : undefined;
  const endTime = typeof args.endTime === "string" ? args.endTime : undefined;
  const needsEtab = scope === "professeur";

  const total = 3 + (needsEtab ? 1 : 0) + 1; // date, durée, motif, [etab], details
  let step = 1;
  const label = (body: string) => wizardStep(step, Math.max(total, step), body);

  const draft = (): Record<string, unknown> => ({
    scope,
    etablissement,
    periodType: periodType || undefined,
    startDate: startDate === WIZARD_DATE_OTHER ? "" : startDate,
    date: startDate === WIZARD_DATE_OTHER ? "" : startDate,
    endDate: endDate === WIZARD_DATE_OTHER ? "" : endDate,
    startTime,
    endTime,
    reason,
    details,
    ...(detailsResolved ? { detailsResolved: true } : {}),
  });

  // 1 — Date début
  if (startDate === WIZARD_DATE_OTHER) {
    return choicesResult(
      "create_absence",
      "startDate",
      label("Choisissez la date d'absence dans le calendrier :"),
      [],
      { ...draft(), startDate: "", date: "" },
      "date",
    );
  }
  if (!startDate || !/^\d{4}-\d{2}-\d{2}$/.test(startDate)) {
    return choicesResult(
      "create_absence",
      "startDate",
      label("Demandons une autorisation d'absence. Quel jour ?"),
      buildDateQuickOptions(calendarDateKeyParis()),
      draft(),
    );
  }
  step += 1;

  // 2 — Une journée / plusieurs
  if (!periodType) {
    return choicesResult(
      "create_absence",
      "periodType",
      label(`Absence le ${weekdayLabelFr(startDate)} — durée ?`),
      [
        { value: "single_day", label: "Une seule journée" },
        { value: "multi_day", label: "Plusieurs jours" },
      ],
      draft(),
    );
  }
  step += 1;

  if (periodType === "multi_day") {
    if (endDate === WIZARD_DATE_OTHER) {
      return choicesResult(
        "create_absence",
        "endDate",
        label("Choisissez la date de fin :"),
        [],
        { ...draft(), endDate: "" },
        "date",
      );
    }
    if (!endDate || !/^\d{4}-\d{2}-\d{2}$/.test(endDate)) {
      return choicesResult(
        "create_absence",
        "endDate",
        label("Jusqu'à quelle date êtes-vous absent(e) ?"),
        buildDateQuickOptions(startDate),
        draft(),
      );
    }
    if (endDate < startDate) {
      return choicesResult(
        "create_absence",
        "endDate",
        label("La date de fin doit être après le début. Choisissez à nouveau :"),
        buildDateQuickOptions(startDate),
        { ...draft(), endDate: "" },
      );
    }
    step += 1;
  } else {
    endDate = startDate;
  }

  // 3 — Motif
  if (!reason) {
    return choicesResult(
      "create_absence",
      "reason",
      label("Quel est le motif ?"),
      COMMON_REASONS.map((r) => ({ value: r, label: r })),
      draft(),
    );
  }
  if (reason === "Autre") {
    const custom = String(args.reasonOther || details || "").trim();
    if (!custom) {
      return choicesResult(
        "create_absence",
        "reasonOther",
        label("Précisez le motif :"),
        [],
        draft(),
        "text",
      );
    }
    reason = custom;
  }
  step += 1;

  // 4 — Établissement (prof)
  if (needsEtab && !etablissement) {
    return choicesResult(
      "create_absence",
      "etablissement",
      label("Pour quel établissement (votre service) ?"),
      etabChoices,
      draft(),
    );
  }
  if (needsEtab) step += 1;

  // 5 — Détails optionnels
  if (!detailsResolved && !details) {
    return choicesResult(
      "create_absence",
      "details",
      label("Un détail à ajouter ? (facultatif — laissez « Non » si rien)"),
      [
        { value: "Non", label: "Non, rien à ajouter" },
        { value: "__CUSTOM__", label: "Oui, saisir un commentaire…" },
      ],
      draft(),
    );
  }
  let finalDetails = details;
  if (details === "__CUSTOM__") {
    const custom = String(args.detailsCustom || "").trim();
    if (!custom) {
      return choicesResult(
        "create_absence",
        "detailsCustom",
        label("Votre commentaire :"),
        [],
        { ...draft(), details: "__CUSTOM__" },
        "text",
      );
    }
    finalDetails = custom;
  } else if (details === "Non") {
    finalDetails = "";
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
        details: finalDetails,
        detailsResolved: true,
      },
      summaryFr:
        `Récap — Demande d'autorisation d'absence le ${weekdayLabelFr(startDate)}` +
        (endDate !== startDate ? ` → ${weekdayLabelFr(endDate)}` : "") +
        ` — motif : ${reason}` +
        (etablissement ? ` (${etablissement})` : "") +
        " ?",
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
      details: finalDetails,
    },
    workflowStatus: "OUVERTE",
    managerDecision: "EN_ATTENTE",
    closedAt: null,
    justificatifRelanceAt: null,
    makeupSlotsRelanceAt: null,
    justification: null,
    history: [
      {
        at: now,
        by: creatorName,
        action: "CREATION",
        note: "Demande d'autorisation d'absence créée via ScolIA (wizard)",
      },
    ],
  };

  const index = await purgeExpiredAbsences(await getAbsenceIndex());
  const { record: saved, merged } = await saveOrMergeAbsenceRecord(
    index,
    record,
    creatorName,
  );

  return {
    ok: true,
    data: {
      id: saved.id,
      merged: Boolean(merged),
      followUrl: "/rh?tab=absences",
    },
    summaryFr: merged
      ? `Demande fusionnée avec une demande existante (${saved.id}).`
      : `Demande d'autorisation d'absence envoyée (${saved.id}). Vous pourrez ajouter un justificatif dans RH.`,
  };
}
