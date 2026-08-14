/**
 * Vacances scolaires France métropolitaine (zones A / B / C).
 * Normandie = Zone B.
 *
 * Convention : début = samedi (après la classe la veille) ; fin = lundi de reprise (matin).
 * Un jour est « en vacances » si date >= début et date < fin (reprise exclue).
 */

export type SchoolHolidayZone = "A" | "B" | "C";

type SchoolHolidayPeriod = {
  id: string;
  label: string;
  /** Inclusive YYYY-MM-DD (souvent un samedi). */
  start: string;
  /** Exclusive YYYY-MM-DD (lundi de reprise). */
  endExclusive: string;
  zones: SchoolHolidayZone[] | "all";
};

/**
 * Fallback lib uniquement (tests / appels sans config).
 * L’établissement doit définir `schoolHolidayZone` dans settings/site.json —
 * l’UI ne présélectionne plus cette valeur.
 */
const DEFAULT_SCHOOL_HOLIDAY_ZONE: SchoolHolidayZone = "B";

const SCHOOL_HOLIDAY_ZONE_OPTIONS: { value: SchoolHolidayZone; label: string; hint: string }[] =
  [
    { value: "A", label: "Zone A", hint: "ex. Lyon, Clermont-Ferrand, Montpellier…" },
    { value: "B", label: "Zone B", hint: "ex. Normandie, Lille, Nantes, Rennes…" },
    { value: "C", label: "Zone C", hint: "ex. Paris, Versailles, Créteil…" },
  ];

/**
 * Calendriers officiels 2025-2026 et 2026-2027 (sources MEN / Service-Public).
 * Hiver / printemps varient selon la zone ; Toussaint, Noël, Été sont communs.
 */
const SCHOOL_HOLIDAY_PERIODS: SchoolHolidayPeriod[] = [
  // —— 2025-2026 ——
  {
    id: "2025-toussaint",
    label: "Vacances de la Toussaint",
    start: "2025-10-18",
    endExclusive: "2025-11-03",
    zones: "all",
  },
  {
    id: "2025-noel",
    label: "Vacances de Noël",
    start: "2025-12-20",
    endExclusive: "2026-01-05",
    zones: "all",
  },
  {
    id: "2026-hiver-A",
    label: "Vacances d'hiver",
    start: "2026-02-07",
    endExclusive: "2026-02-23",
    zones: ["A"],
  },
  {
    id: "2026-hiver-B",
    label: "Vacances d'hiver",
    start: "2026-02-14",
    endExclusive: "2026-03-02",
    zones: ["B"],
  },
  {
    id: "2026-hiver-C",
    label: "Vacances d'hiver",
    start: "2026-02-21",
    endExclusive: "2026-03-09",
    zones: ["C"],
  },
  {
    id: "2026-printemps-A",
    label: "Vacances de printemps",
    start: "2026-04-04",
    endExclusive: "2026-04-20",
    zones: ["A"],
  },
  {
    id: "2026-printemps-B",
    label: "Vacances de printemps",
    start: "2026-04-11",
    endExclusive: "2026-04-27",
    zones: ["B"],
  },
  {
    id: "2026-printemps-C",
    label: "Vacances de printemps",
    start: "2026-04-18",
    endExclusive: "2026-05-04",
    zones: ["C"],
  },
  {
    id: "2026-ete",
    label: "Vacances d'été",
    start: "2026-07-04",
    endExclusive: "2026-09-01",
    zones: "all",
  },
  // —— 2026-2027 ——
  {
    id: "2026-toussaint",
    label: "Vacances de la Toussaint",
    start: "2026-10-17",
    endExclusive: "2026-11-02",
    zones: "all",
  },
  {
    id: "2026-noel",
    label: "Vacances de Noël",
    start: "2026-12-19",
    endExclusive: "2027-01-04",
    zones: "all",
  },
  {
    id: "2027-hiver-A",
    label: "Vacances d'hiver",
    start: "2027-02-13",
    endExclusive: "2027-03-01",
    zones: ["A"],
  },
  {
    id: "2027-hiver-B",
    label: "Vacances d'hiver",
    start: "2027-02-20",
    endExclusive: "2027-03-08",
    zones: ["B"],
  },
  {
    id: "2027-hiver-C",
    label: "Vacances d'hiver",
    start: "2027-02-06",
    endExclusive: "2027-02-22",
    zones: ["C"],
  },
  {
    id: "2027-printemps-A",
    label: "Vacances de printemps",
    start: "2027-04-10",
    endExclusive: "2027-04-26",
    zones: ["A"],
  },
  {
    id: "2027-printemps-B",
    label: "Vacances de printemps",
    start: "2027-04-17",
    endExclusive: "2027-05-03",
    zones: ["B"],
  },
  {
    id: "2027-printemps-C",
    label: "Vacances de printemps",
    start: "2027-04-03",
    endExclusive: "2027-04-19",
    zones: ["C"],
  },
  {
    id: "2027-ete",
    label: "Vacances d'été",
    start: "2027-07-03",
    endExclusive: "2027-09-01",
    zones: "all",
  },
];

function periodApplies(p: SchoolHolidayPeriod, zone: SchoolHolidayZone) {
  return p.zones === "all" || p.zones.includes(zone);
}

export function schoolHolidayOnDate(
  isoDate: string,
  zone?: SchoolHolidayZone | null,
): SchoolHolidayPeriod | null {
  if (!zone) return null;
  for (const p of SCHOOL_HOLIDAY_PERIODS) {
    if (!periodApplies(p, zone)) continue;
    if (isoDate >= p.start && isoDate < p.endExclusive) return p;
  }
  return null;
}

function isSchoolHolidayDate(
  isoDate: string,
  zone?: SchoolHolidayZone | null,
): boolean {
  return schoolHolidayOnDate(isoDate, zone) != null;
}

export function isWeekendIsoDate(isoDate: string): boolean {
  const d = new Date(`${isoDate}T12:00:00`);
  if (Number.isNaN(d.getTime())) return false;
  const js = d.getDay();
  return js === 0 || js === 6;
}
