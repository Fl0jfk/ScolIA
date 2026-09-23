/**
 * Absences RH validées → LeaveSpan (trou EDT / mon-planning).
 * Pure — pas d’I/O. Les congés `personnel_leave` restent une autre source.
 */

import type { AbsenceRecord } from "@/app/lib/absences-types";
import type { LeaveSpan } from "@/app/lib/rh/planning-calendar";

export type AbsenceLeaveLike = Pick<
  AbsenceRecord,
  "id" | "managerDecision" | "calendarVisible" | "personnelId" | "displayName"
> & {
  createdBy?: { userId?: string } | null;
  data: {
    startDate: string;
    endDate: string;
    reason?: string | null;
  };
  hoursTreatment?: string | null;
};

/** Absence direction VALIDEE et visible calendrier → candidat trou EDT. */
export function isValidatedAbsenceForEdtHole(record: AbsenceLeaveLike): boolean {
  return record.managerDecision === "VALIDEE" && record.calendarVisible === true;
}

export function absenceMatchesPersonnel(
  record: AbsenceLeaveLike,
  opts: { personnelId?: string | null; userId?: string | null },
): boolean {
  const pid = opts.personnelId?.trim() || "";
  const uid = opts.userId?.trim() || "";
  if (pid && record.personnelId && record.personnelId === pid) return true;
  if (uid && record.createdBy?.userId && record.createdBy.userId === uid) return true;
  return false;
}

function leaveTypeFromAbsence(record: AbsenceLeaveLike): string {
  const t = String(record.hoursTreatment || "").trim();
  if (t === "MALADIE") return "absence_maladie";
  if (t === "ENFANT_MALADE") return "absence_enfant_malade";
  if (t === "DEDUCTION_SALAIRE") return "absence_deduction";
  if (t) return `absence_${t.toLowerCase()}`;
  return "absence_rh";
}

function leaveLabelFromAbsence(record: AbsenceLeaveLike): string {
  const reason = String(record.data.reason || "").trim();
  if (reason) return `Absence — ${reason}`;
  return "Absence RH validée";
}

/** Une absence → LeaveSpan (ou null si hors critères). */
export function absenceToLeaveSpan(record: AbsenceLeaveLike): LeaveSpan | null {
  if (!isValidatedAbsenceForEdtHole(record)) return null;
  const startDate = String(record.data.startDate || "").slice(0, 10);
  const endDate = String(record.data.endDate || record.data.startDate || "").slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(startDate) || !/^\d{4}-\d{2}-\d{2}$/.test(endDate)) {
    return null;
  }
  if (endDate < startDate) return null;
  return {
    startDate,
    endDate,
    type: leaveTypeFromAbsence(record),
    label: leaveLabelFromAbsence(record),
  };
}

/** Fusionne congés RH + absences validées (dédup par plage+type). */
export function mergeLeaveSpans(...groups: LeaveSpan[][]): LeaveSpan[] {
  const seen = new Set<string>();
  const out: LeaveSpan[] = [];
  for (const group of groups) {
    for (const span of group) {
      const key = `${span.startDate}|${span.endDate}|${span.type}|${span.label}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(span);
    }
  }
  return out;
}

export function absencesToLeaveSpans(
  records: AbsenceLeaveLike[],
  opts?: { personnelId?: string | null; userId?: string | null },
): LeaveSpan[] {
  const scoped = opts
    ? records.filter((r) => absenceMatchesPersonnel(r, opts))
    : records;
  const spans: LeaveSpan[] = [];
  for (const r of scoped) {
    const span = absenceToLeaveSpan(r);
    if (span) spans.push(span);
  }
  return spans;
}
