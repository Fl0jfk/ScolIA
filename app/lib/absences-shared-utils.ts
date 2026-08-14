import { resolveAbsenceScope, type AbsenceRecord } from "@/app/lib/absences-types";

export function normalizeAbsencePersonName(name: string): string {
  return String(name || "")
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, " ");
}

export function absencePersonNamesMatch(a: string, b: string): boolean {
  const na = normalizeAbsencePersonName(a);
  const nb = normalizeAbsencePersonName(b);
  if (!na || !nb) return false;
  if (na === nb) return true;
  const ta = [...na.split(" ")].sort().join(" ");
  const tb = [...nb.split(" ")].sort().join(" ");
  return ta === tb;
}

export function isPendingAbsenceRecord(record: AbsenceRecord): boolean {
  return record.managerDecision === "EN_ATTENTE" && record.workflowStatus !== "CLOTUREE";
}

export function compareAbsenceRecordsAlphabetically(a: AbsenceRecord, b: AbsenceRecord): number {
  const ogecA = resolveAbsenceScope(a) === "ogec";
  const ogecB = resolveAbsenceScope(b) === "ogec";
  if (ogecA !== ogecB) return ogecA ? -1 : 1;
  const nameA = a.displayName || a.createdBy.name || "";
  const nameB = b.displayName || b.createdBy.name || "";
  return nameA.localeCompare(nameB, "fr", { sensitivity: "base" });
}

function sortAbsenceRecordsAlphabetically<T extends AbsenceRecord>(items: T[]): T[] {
  return [...items].sort(compareAbsenceRecordsAlphabetically);
}
