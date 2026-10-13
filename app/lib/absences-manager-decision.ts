import "server-only";

/**
 * Décision direction / validateur OGEC sur une absence RH (VALIDER / REFUSER).
 * Même logique métier que PATCH /api/absences — factorisée pour ScolIA.
 */

import {
  autoCloseAdminNote,
  autoClosesAfterDirectionValidation,
  forcedHoursTreatmentForNonDiscretionaryAbsence,
  formatAbsenceHoursTreatment,
  hasMakeupSlotsInfo,
  isNonDiscretionaryAbsence,
  isNonDiscretionaryTreatment,
  isRattrapageTreatment,
  isRhRetardAbsence,
  isSansIncidenceTreatment,
  reasonLabelForNonDiscretionaryTreatment,
  validateHoursTreatmentForAbsence,
  type AbsenceHoursTreatment,
} from "@/app/lib/absence-hours-treatment";
import {
  applyPostValidationPrivacy,
  getAbsenceIndex,
  getAbsenceRecord,
  saveAbsenceRecord,
} from "@/app/lib/absences-storage";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { resolveAbsenceScope } from "@/app/lib/absences-types";
import {
  notifyAbsenceCreatorValidated,
  notifyAbsenceMakeupSlotsRequested,
  notifyAbsenceValidated,
} from "@/app/lib/absences-workflow-mail";

export type ManagerDecisionAction = "VALIDER" | "REFUSER";

export type ApplyManagerDecisionInput = {
  absenceId: string;
  action: ManagerDecisionAction;
  actorName: string;
  managerNote?: string | null;
  hoursTreatment?: unknown;
  directionConfirmedMakeupSlots?: string | null;
};

export type ApplyManagerDecisionResult =
  | { ok: true; record: AbsenceRecord; validationRecipients?: string[] }
  | { ok: false; error: string };

export async function applyAbsenceManagerDecision(
  input: ApplyManagerDecisionInput,
): Promise<ApplyManagerDecisionResult> {
  const current = await getAbsenceRecord(input.absenceId);
  if (!current) return { ok: false, error: "Absence introuvable." };

  if (current.managerDecision !== "EN_ATTENTE" || current.workflowStatus === "CLOTUREE") {
    return { ok: false, error: "Cette absence n’est plus en attente de validation." };
  }

  const actor = input.actorName.trim() || "Direction";
  const managerNote = String(input.managerNote || "").trim();
  let updated: AbsenceRecord;
  let validationRecipients: string[] | undefined;

  if (input.action === "VALIDER") {
    const forcedMedical = forcedHoursTreatmentForNonDiscretionaryAbsence(current);
    let hoursTreatment;
    if (forcedMedical) {
      hoursTreatment = forcedMedical;
    } else {
      const treatmentResult = validateHoursTreatmentForAbsence(
        resolveAbsenceScope(current),
        current.data.etablissement,
        input.hoursTreatment,
        { isRetard: isRhRetardAbsence(current) },
      );
      if (!treatmentResult.ok) {
        return { ok: false, error: treatmentResult.error };
      }
      hoursTreatment = treatmentResult.treatment;
    }

    const directionConfirmedMakeupSlots =
      isNonDiscretionaryTreatment(hoursTreatment) || isSansIncidenceTreatment(hoursTreatment)
        ? null
        : input.directionConfirmedMakeupSlots
          ? String(input.directionConfirmedMakeupSlots).trim() || null
          : null;

    const decidedAt = new Date().toISOString();
    const isMakeup =
      isRattrapageTreatment(hoursTreatment) &&
      !hasMakeupSlotsInfo({
        staffPreferredMakeupSlots: current.staffPreferredMakeupSlots,
        directionConfirmedMakeupSlots,
      });
    const scope = resolveAbsenceScope(current);
    const autoClose = autoClosesAfterDirectionValidation(scope, hoursTreatment);
    const closeNote = autoCloseAdminNote(hoursTreatment);

    updated = {
      ...current,
      managerNote,
      updatedAt: decidedAt,
      managerDecision: "VALIDEE",
      workflowStatus: autoClose
        ? "CLOTUREE"
        : current.justification?.fileUrl
          ? "JUSTIFICATIF_DEPOSE"
          : "A_TRAITER",
      calendarVisible: true,
      closedAt: autoClose ? decidedAt : null,
      hoursTreatment,
      ...(forcedMedical
        ? {
            staffPreferredTreatment: forcedMedical,
            data: {
              ...current.data,
              reason:
                current.data.reason?.trim() ||
                reasonLabelForNonDiscretionaryTreatment(forcedMedical),
            },
            staffPreferredMakeupSlots: null,
          }
        : {}),
      directionConfirmedMakeupSlots,
      makeupSlotsRelanceAt: isMakeup ? decidedAt : null,
      adminTreatedAt: autoClose ? decidedAt : null,
      adminTreatedBy: autoClose ? actor : null,
      adminNote: autoClose ? closeNote : current.adminNote ?? null,
      history: [
        ...(current.history || []),
        {
          at: decidedAt,
          by: actor,
          action: "DECISION_VALIDEE",
          note:
            managerNote ||
            (forcedMedical
              ? `Prise d'acte direction — ${reasonLabelForNonDiscretionaryTreatment(forcedMedical).toLowerCase()}.`
              : isSansIncidenceTreatment(hoursTreatment)
                ? "Prise d’acte direction — retard sans suite."
                : undefined),
        },
        ...(autoClose
          ? [
              {
                at: decidedAt,
                by: actor,
                action: "TRAITEMENT_ADMIN",
                note: closeNote,
              },
            ]
          : []),
        ...(isMakeup
          ? [
              {
                at: decidedAt,
                by: actor,
                action: "RELANCE_CRENEAUX_RATTRAPAGE",
                note: "Relance automatique : créneaux de rattrapage non indiqués.",
              },
            ]
          : []),
      ],
    };
  } else {
    if (isNonDiscretionaryAbsence(current)) {
      return {
        ok: false,
        error:
          "Les absences maladie ou enfant malade ne peuvent pas être refusées. La direction prend acte (valider), puis le dossier part en traitement.",
      };
    }
    const closedAt = new Date().toISOString();
    updated = {
      ...current,
      managerNote,
      updatedAt: closedAt,
      managerDecision: "REFUSEE",
      workflowStatus: "CLOTUREE",
      calendarVisible: false,
      closedAt,
      justificatifRelanceAt: null,
      makeupSlotsRelanceAt: null,
      history: [
        ...(current.history || []),
        {
          at: closedAt,
          by: actor,
          action: "DECISION_REFUSEE",
          note: managerNote || undefined,
        },
      ],
    };
  }

  await saveAbsenceRecord(updated);

  if (input.action === "VALIDER") {
    if (updated.workflowStatus === "CLOTUREE" && updated.adminTreatedAt) {
      try {
        const index = await getAbsenceIndex();
        updated = await applyPostValidationPrivacy(updated, index);
        await saveAbsenceRecord(updated);
      } catch (privErr) {
        console.error("[absences-manager-decision] privacy:", privErr);
      }
    }
    try {
      const { recipients } = await notifyAbsenceValidated(updated);
      validationRecipients = recipients;
    } catch (mailErr) {
      console.error("[absences-manager-decision] mail validated:", mailErr);
      validationRecipients = [];
    }
    try {
      await notifyAbsenceCreatorValidated(updated);
    } catch (mailErr) {
      console.error("[absences-manager-decision] mail creator:", mailErr);
    }
    if (
      isRattrapageTreatment(updated.hoursTreatment) &&
      !hasMakeupSlotsInfo(updated) &&
      updated.makeupSlotsRelanceAt
    ) {
      try {
        await notifyAbsenceMakeupSlotsRequested({
          record: updated,
          fromProcessor: false,
        });
      } catch (mailErr) {
        console.error("[absences-manager-decision] makeup relance:", mailErr);
      }
    }
  }

  return { ok: true, record: updated, validationRecipients };
}

export function summarizeAbsenceForManager(abs: AbsenceRecord): string {
  const name = abs.displayName || abs.createdBy?.name || "Agent";
  const reason = abs.data.reason || "—";
  const dates =
    abs.data.startDate && abs.data.endDate && abs.data.startDate !== abs.data.endDate
      ? `${abs.data.startDate} → ${abs.data.endDate}`
      : abs.data.startDate || "?";
  const treatment =
    formatAbsenceHoursTreatment(
      (abs.staffPreferredTreatment as AbsenceHoursTreatment | null | undefined) ?? null,
    ) || formatAbsenceHoursTreatment(abs.hoursTreatment ?? null);
  const scope = resolveAbsenceScope(abs);
  const etab = abs.data.etablissement ? ` · ${abs.data.etablissement}` : "";
  return `${name} — ${dates} — ${reason}${treatment ? ` (${treatment})` : ""} [${scope}${etab}]`;
}
