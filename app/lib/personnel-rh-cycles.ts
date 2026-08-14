import type { PersonnelEntretien, PersonnelMedecineTravail, PersonnelRecord } from "@/app/lib/personnel-types";
import { daysUntil, isOverdue, uid } from "@/app/lib/personnel-types";

export const RH_MEDECINE_INTERVAL_YEARS = 3;
const RH_ENTRETIEN_INTERVAL_YEARS = 3;

function parseIsoDate(iso?: string | null): Date | null {
  if (!iso) return null;
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? null : d;
}

function toIsoDateOnly(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function addYears(isoDate: string, years: number): string {
  const d = parseIsoDate(isoDate);
  if (!d) return isoDate;
  const next = new Date(d);
  next.setFullYear(next.getFullYear() + years);
  return toIsoDateOnly(next);
}

function calendarYearBounds(year: number) {
  return {
    start: new Date(year, 0, 1),
    end: new Date(year, 11, 31, 23, 59, 59, 999),
  };
}

function isDueInCalendarYear(dueDate: string | null | undefined, year: number): boolean {
  const d = parseIsoDate(dueDate);
  if (!d) return false;
  return d.getFullYear() === year;
}

function isDueInOrBeforeCalendarYear(dueDate: string | null | undefined, year: number): boolean {
  const d = parseIsoDate(dueDate);
  if (!d) return false;
  return d.getFullYear() <= year;
}

export function computeNextMedecineDue(lastVisitAt: string): string {
  return addYears(lastVisitAt, RH_MEDECINE_INTERVAL_YEARS);
}

export function computeNextEntretienDue(lastCompletedAt: string): string {
  return addYears(lastCompletedAt, RH_ENTRETIEN_INTERVAL_YEARS);
}

/** Recalcule lastVisitAt / nextVisitAt depuis l'historique des visites. */
export function syncMedecineDerivedFields(med: PersonnelMedecineTravail): PersonnelMedecineTravail {
  const visits = [...(med.visits || [])].sort(
    (a, b) => new Date(b.visitedAt).getTime() - new Date(a.visitedAt).getTime(),
  );
  const latest = visits[0];
  if (!latest) {
    return { ...med, visits, lastVisitAt: med.lastVisitAt ?? null, nextVisitAt: med.nextVisitAt ?? null };
  }
  return {
    ...med,
    visits,
    lastVisitAt: latest.visitedAt,
    nextVisitAt: computeNextMedecineDue(latest.visitedAt),
    visitType: latest.visitType || med.visitType,
  };
}

export function normalizeMedecineTravail(raw: PersonnelMedecineTravail | undefined | null): PersonnelMedecineTravail {
  const med = raw && typeof raw === "object" ? raw : { visits: [] };
  let visits = Array.isArray(med.visits) ? [...med.visits] : [];

  if (visits.length === 0 && med.lastVisitAt) {
    visits = [
      {
        id: uid("medv"),
        visitedAt: med.lastVisitAt,
        visitType: med.visitType,
        documentId: med.documentId ?? null,
        notes: med.notes,
        createdAt: med.lastVisitAt,
      },
    ];
  }

  return syncMedecineDerivedFields({
    visits,
    lastVisitAt: med.lastVisitAt ?? null,
    nextVisitAt: med.nextVisitAt ?? null,
    visitType: med.visitType || "",
    notes: med.notes || "",
    documentId: med.documentId ?? null,
  });
}

function lastCompletedEntretien(record: PersonnelRecord): PersonnelEntretien | null {
  const done = record.entretiens
    .filter((e) => e.status === "realise" && e.completedAt)
    .sort((a, b) => new Date(b.completedAt!).getTime() - new Date(a.completedAt!).getTime());
  return done[0] || null;
}

export function computeEntretienNextDue(record: PersonnelRecord): string | null {
  const last = lastCompletedEntretien(record);
  if (!last?.completedAt) return null;
  return computeNextEntretienDue(last.completedAt);
}

export function hasOpenEntretienForCycle(record: PersonnelRecord, year: number): boolean {
  const bounds = calendarYearBounds(year);
  return record.entretiens.some((e) => {
    if (e.status === "realise") {
      const c = parseIsoDate(e.completedAt);
      return c && c >= bounds.start && c <= bounds.end;
    }
    if (e.status === "planifie" && e.scheduledAt) {
      const s = parseIsoDate(e.scheduledAt);
      return s && s >= bounds.start && s <= bounds.end;
    }
    return false;
  });
}

export function entretienDueThisYear(record: PersonnelRecord, year = new Date().getFullYear()): boolean {
  if (!record.active) return false;
  const nextDue = computeEntretienNextDue(record);
  if (!nextDue) {
    // Jamais d'entretien réalisé : à positionner si embauché avant fin d'année
    return true;
  }
  if (!isDueInOrBeforeCalendarYear(nextDue, year)) return false;
  return !hasOpenEntretienForCycle(record, year);
}

export function medecineDueThisYear(record: PersonnelRecord, year = new Date().getFullYear()): boolean {
  if (!record.active) return false;
  const next = record.medecineTravail.nextVisitAt;
  if (!next) return true;
  if (isOverdue(next)) return true;
  return isDueInCalendarYear(next, year);
}

export function formatDueLabel(dueDate: string | null | undefined): string {
  const d = daysUntil(dueDate);
  if (d === null) return "Date à renseigner";
  if (d < 0) return `En retard de ${Math.abs(d)} j`;
  if (d === 0) return "Aujourd'hui";
  return `Dans ${d} j`;
}

export function mapExtractedDocCategory(typeDocument?: string): import("@/app/lib/personnel-types").PersonnelDocCategory {
  const t = (typeDocument || "").toLowerCase();
  if (t.includes("medecin") || t.includes("médical") || t.includes("santé") || t.includes("visite")) return "medecine";
  if (t.includes("entretien")) return "entretien";
  if (t.includes("contrat") || t.includes("embauche")) return "contrat";
  if (t.includes("formation")) return "formation";
  if (t.includes("habilitation") || t.includes("caces") || t.includes("sst")) return "habilitation";
  if (t.includes("onboarding") || t.includes("intégration")) return "onboarding";
  return "autre";
}
