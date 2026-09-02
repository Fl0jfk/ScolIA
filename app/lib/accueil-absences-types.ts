/**
 * Types et helpers purs — saisie absences à l’accueil (élèves + staff).
 * Pas de I/O : utilisable client et tests.
 */

export const ACCUEIL_ABSENCES_MODULE_ID = "accueil-absences";
/** Consultation seule — absences déclarées à l’accueil (CPE, surveillants). */
export const ABSENCES_ACCUEIL_CONSULTATION_MODULE_ID = "absences-accueil-consultation";

export type AccueilPersonKind = "eleve" | "enseignant" | "personnel";
export type AccueilStaffScope = "professeur" | "ogec";
export type AccueilAbsenceCanal = "telephone" | "physique" | "mail";
export type AccueilPeriodMode = "today" | "hours" | "multi_day";
/** Nature élève à l’accueil : journée / créneau (absence) ou retard signalé. */
export type AccueilEleveNature = "absence" | "retard";
export type AccueilVsSource = "appel" | "accueil" | "famille" | "charlemagne";

export type AccueilSearchHit = {
  kind: AccueilPersonKind;
  id: string;
  nom: string;
  prenom: string;
  displayName: string;
  subtitle: string;
  cycle: "ecole" | "college" | "lycee" | null;
  classe?: string | null;
  scope?: AccueilStaffScope;
  category?: string | null;
};

export type AccueilBoardKind = "eleve" | "professeur" | "ogec";

export type AccueilBoardRow = {
  id: string;
  kind: AccueilBoardKind;
  displayName: string;
  subtitle: string;
  dateDebut: string;
  dateFin: string;
  heureDebut: string | null;
  heureFin: string | null;
  motif: string | null;
  createdByNom: string | null;
  source: "accueil";
  /** Élèves : absence ou retard. */
  eleveNature?: AccueilEleveNature | null;
};

function parseHm(raw: string | null | undefined): number | null {
  const m = String(raw || "")
    .trim()
    .match(/^(\d{1,2}):(\d{2})/);
  if (!m) return null;
  return Number(m[1]) * 60 + Number(m[2]);
}

export function timesOverlap(
  aStart: string | null | undefined,
  aEnd: string | null | undefined,
  bStart: string | null | undefined,
  bEnd: string | null | undefined,
): boolean {
  const as = parseHm(aStart);
  const ae = parseHm(aEnd);
  const bs = parseHm(bStart);
  const be = parseHm(bEnd);
  if (as == null || ae == null || bs == null || be == null) return true;
  return as < be && bs < ae;
}

export function datesOverlap(aStart: string, aEnd: string, bStart: string, bEnd: string): boolean {
  return aStart <= bEnd && bStart <= aEnd;
}

/** Une absence journée / créneau couvre-t-elle un créneau d’appel ? */
export function absenceCoversSlot(input: {
  dateDebut: string;
  dateFin: string;
  heureDebut?: string | null;
  heureFin?: string | null;
  slotDate: string;
  slotHeureDebut?: string | null;
  slotHeureFin?: string | null;
}): boolean {
  if (!datesOverlap(input.dateDebut, input.dateFin, input.slotDate, input.slotDate)) {
    return false;
  }
  return timesOverlap(
    input.heureDebut,
    input.heureFin,
    input.slotHeureDebut,
    input.slotHeureFin,
  );
}

export function cycleLabel(cycle: "ecole" | "college" | "lycee" | null | undefined): string {
  if (cycle === "ecole") return "École";
  if (cycle === "college") return "Collège";
  if (cycle === "lycee") return "Lycée";
  return "";
}

export function normalizeSearchNeedle(raw: string): string {
  return raw
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

export function escapeLike(raw: string): string {
  return raw.replace(/[%_\\]/g, (ch) => `\\${ch}`);
}

export function asCycle(value: unknown): "ecole" | "college" | "lycee" | null {
  const s = String(value ?? "")
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
  if (s === "ecole" || s.includes("primaire") || s.includes("elementaire") || s.includes("maternelle")) {
    return "ecole";
  }
  if (s === "college") return "college";
  if (s === "lycee") return "lycee";
  return null;
}
