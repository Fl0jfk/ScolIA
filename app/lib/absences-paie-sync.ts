/**
 * Post-VALIDER : résoudre personnel + créer `paie_element` (nature absence) si pertinent.
 * Idempotent sur `absence_id`. Ne crée pas de période silencieusement.
 */

import "server-only";

import type { AbsenceHoursTreatment } from "@/app/lib/absence-hours-treatment";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { absenceShouldCreatePaieElement } from "@/app/lib/absences-paie-sync-rules";
import {
  ensurePaieElementFromAbsence,
  findOpenPaiePeriodeForDate,
  type PaieElementRow,
} from "@/app/lib/paie-etablissement-db";
import {
  findPersonnelByEmail,
  findPersonnelByExternalId,
} from "@/app/lib/personnel-storage";

export { absenceShouldCreatePaieElement } from "@/app/lib/absences-paie-sync-rules";

export type AbsencePaieSyncResult =
  | { ok: true; status: "created"; element: PaieElementRow; personnelId: string }
  | { ok: true; status: "exists"; element: PaieElementRow; personnelId: string }
  | {
      ok: true;
      status: "skipped";
      reason:
        | "not_validated"
        | "rattrapage"
        | "no_personnel"
        | "no_open_periode"
        | "no_start_date";
      personnelId?: string | null;
    }
  | { ok: false; error: string };

export async function resolveAbsencePersonnelId(
  record: AbsenceRecord,
): Promise<string | null> {
  const existing = String(record.personnelId || "").trim();
  if (existing) return existing;

  const userId = String(record.createdBy?.userId || "").trim();
  if (userId) {
    const byExt = await findPersonnelByExternalId(userId);
    if (byExt?.id) return byExt.id;
  }

  const email = String(record.createdBy?.email || "").trim();
  if (email) {
    const byEmail = await findPersonnelByEmail(email);
    if (byEmail?.id) return byEmail.id;
  }

  return null;
}

function buildAbsencePaieLibelle(record: AbsenceRecord): string {
  const reason = String(record.data.reason || "").trim() || "Absence";
  const start = String(record.data.startDate || "").slice(0, 10);
  const end = String(record.data.endDate || start).slice(0, 10);
  const range = start && end && end !== start ? `${start} → ${end}` : start || "";
  const treatment = String(record.hoursTreatment || "").trim();
  const suffix = treatment ? ` (${treatment})` : "";
  const base = range ? `${reason} — ${range}${suffix}` : `${reason}${suffix}`;
  return base.slice(0, 200);
}

function estimateAbsenceQuantiteHours(record: AbsenceRecord): number | null {
  const start = new Date(record.data.startAt);
  const end = new Date(record.data.endAt);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) return null;
  const ms = end.getTime() - start.getTime();
  if (ms <= 0) return null;
  const hours = ms / 3_600_000;
  const days =
    Math.floor(
      (Date.UTC(end.getFullYear(), end.getMonth(), end.getDate()) -
        Date.UTC(start.getFullYear(), start.getMonth(), start.getDate())) /
        86_400_000,
    ) + 1;
  if (days > 1) {
    return Math.min(hours, days * 7);
  }
  return Math.round(hours * 100) / 100;
}

/**
 * Après validation direction : crée l’élément paie si période brouillon ouverte.
 * Retourne aussi le `personnelId` résolu pour backfill éventuel sur l’absence.
 */
export async function syncPaieElementAfterAbsenceValidation(
  etablissementId: string,
  record: AbsenceRecord,
): Promise<AbsencePaieSyncResult> {
  if (record.managerDecision !== "VALIDEE") {
    return { ok: true, status: "skipped", reason: "not_validated" };
  }
  if (!absenceShouldCreatePaieElement(record.hoursTreatment)) {
    return { ok: true, status: "skipped", reason: "rattrapage" };
  }

  const personnelId = await resolveAbsencePersonnelId(record);
  if (!personnelId) {
    return { ok: true, status: "skipped", reason: "no_personnel", personnelId: null };
  }

  const startDate = String(record.data.startDate || "").slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(startDate)) {
    return { ok: true, status: "skipped", reason: "no_start_date", personnelId };
  }

  const periode = await findOpenPaiePeriodeForDate(etablissementId, startDate);
  if (!periode) {
    return { ok: true, status: "skipped", reason: "no_open_periode", personnelId };
  }

  try {
    const result = await ensurePaieElementFromAbsence(etablissementId, {
      absenceId: record.id,
      periodeId: periode.id,
      personnelId,
      libelle: buildAbsencePaieLibelle(record),
      quantite: estimateAbsenceQuantiteHours(record),
      hoursTreatment: (record.hoursTreatment || null) as AbsenceHoursTreatment | null,
    });
    return {
      ok: true,
      status: result.created ? "created" : "exists",
      element: result.element,
      personnelId,
    };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : "sync paie impossible",
    };
  }
}
