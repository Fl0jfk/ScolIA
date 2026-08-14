import { pdfFormatDateFr } from "@/app/lib/pdf-format-numbers";
import {
  computeComptaSheetDerived,
  normalizeComptaRecettesLignes,
  readComptaSheetFromTrip,
  type TravelsComptaSheet,
} from "@/app/lib/travels-compta-sheet";
import type { TravelsTrip } from "@/app/lib/travels-types";

/** Année scolaire APEL : 1er septembre → 15 juillet inclus. */
function comptaApelSchoolYearBounds(now = new Date()) {
  const y = now.getFullYear();
  const m = now.getMonth();
  const day = now.getDate();

  // Septembre → 15 juillet : année N-(N+1) en cours.
  // 16 juillet → août : bascule vers l'année suivante (facturation des voyages à venir).
  let startYear: number;
  if (m >= 8) {
    startYear = y;
  } else if (m < 6 || (m === 6 && day <= 15)) {
    startYear = y - 1;
  } else {
    startYear = y;
  }

  return {
    start: new Date(startYear, 8, 1),
    end: new Date(startYear + 1, 6, 15, 23, 59, 59, 999),
    label: `${startYear}-${startYear + 1}`,
    startIso: `${startYear}-09-01`,
    endIso: `${startYear + 1}-07-15`,
  };
}

export const COMPTA_APEL_ETABLISSEMENTS = [
  "École",
  "Collège",
  "Lycée",
  "Groupe Scolaire",
] as const;

export type ComptaApelEtablissement = string;

function normalizeApelEtablissement(raw: string | null | undefined): ComptaApelEtablissement {
  const s = String(raw || "").trim();
  return s || "Groupe Scolaire";
}

export type ComptaApelTotals = {
  apelCollective: number;
  aidesIndividuelles: number;
  totalApel: number;
  tripCount: number;
  tripsWithApel: number;
};

export type ComptaApelTripCommitment = {
  tripId: string;
  title: string;
  etablissement: ComptaApelEtablissement;
  travelDate: string | null;
  travelDateLabel: string;
  apelCollective: number;
  aidesIndividuelles: number;
  totalApel: number;
  hasComptaSheet: boolean;
};

export type ComptaApelEtablissementGroup = {
  etablissement: ComptaApelEtablissement;
  trips: ComptaApelTripCommitment[];
  totals: ComptaApelTotals;
};

export type ComptaApelSummary = {
  schoolYear: {
    label: string;
    startIso: string;
    endIso: string;
  };
  currentTripId: string | null;
  currentTrip: ComptaApelTripCommitment | null;
  byEtablissement: ComptaApelEtablissementGroup[];
  trips: ComptaApelTripCommitment[];
  totals: ComptaApelTotals;
};

function roundApelMoney(n: number): number {
  return Math.round(n * 100) / 100;
}

function computeApelTotals(rows: ComptaApelTripCommitment[]): ComptaApelTotals {
  const totals = rows.reduce(
    (acc, row) => ({
      apelCollective: acc.apelCollective + row.apelCollective,
      aidesIndividuelles: acc.aidesIndividuelles + row.aidesIndividuelles,
      totalApel: acc.totalApel + row.totalApel,
      tripCount: acc.tripCount + 1,
      tripsWithApel: acc.tripsWithApel + (row.totalApel > 0 ? 1 : 0),
    }),
    { apelCollective: 0, aidesIndividuelles: 0, totalApel: 0, tripCount: 0, tripsWithApel: 0 },
  );
  return {
    apelCollective: roundApelMoney(totals.apelCollective),
    aidesIndividuelles: roundApelMoney(totals.aidesIndividuelles),
    totalApel: roundApelMoney(totals.totalApel),
    tripCount: totals.tripCount,
    tripsWithApel: totals.tripsWithApel,
  };
}

function buildApelGroupsByEtablissement(rows: ComptaApelTripCommitment[]): ComptaApelEtablissementGroup[] {
  const order: string[] = [];
  for (const row of rows) {
    if (!order.includes(row.etablissement)) order.push(row.etablissement);
  }
  return order.map((etablissement) => {
    const trips = rows.filter((row) => row.etablissement === etablissement);
    return { etablissement, trips, totals: computeApelTotals(trips) };
  });
}

function apelCommitmentFromSheet(sheet: TravelsComptaSheet | null | undefined): {
  apelCollective: number;
  aidesIndividuelles: number;
  totalApel: number;
} {
  if (!sheet) {
    return { apelCollective: 0, aidesIndividuelles: 0, totalApel: 0 };
  }
  const recettesLignes = normalizeComptaRecettesLignes(sheet);
  const apelCollective = recettesLignes[0]?.amount ?? 0;
  const aidesIndividuelles =
    sheet.totalAidesIndividuelles ??
    (sheet.aidesIndividuelles ?? []).reduce((sum, row) => sum + (row.amount ?? 0), 0);
  const collective = Number.isFinite(apelCollective) ? apelCollective : 0;
  const individuelles = Number.isFinite(aidesIndividuelles) ? aidesIndividuelles : 0;
  return {
    apelCollective: collective,
    aidesIndividuelles: individuelles,
    totalApel: Math.round((collective + individuelles) * 100) / 100,
  };
}

function formatTravelDateLabel(trip: TravelsTrip): string {
  const raw = trip.data?.startDate || trip.data?.date;
  if (!raw) return "Date à préciser";
  const d = new Date(raw);
  if (Number.isNaN(d.getTime())) return "Date à préciser";
  if (trip.type === "COMPLEX" && trip.data?.endDate) {
    const end = new Date(trip.data.endDate);
    if (!Number.isNaN(end.getTime())) {
      return `${pdfFormatDateFr(d)} au ${pdfFormatDateFr(end)}`;
    }
  }
  return pdfFormatDateFr(d);
}

function parseTripLocalDate(raw: string | null | undefined): Date | null {
  if (!raw) return null;
  const s = String(raw).trim();
  const iso = /^(\d{4})-(\d{2})-(\d{2})/.exec(s);
  if (iso) {
    const d = new Date(Number(iso[1]), Number(iso[2]) - 1, Number(iso[3]));
    return Number.isNaN(d.getTime()) ? null : d;
  }
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** Date de début du voyage (tri + filtre année scolaire). */
function tripTravelStartDate(trip: TravelsTrip): Date | null {
  return parseTripLocalDate(trip.data?.startDate || trip.data?.date || null);
}

function tripInApelSchoolYear(trip: TravelsTrip, start: Date, end: Date): boolean {
  const travel = tripTravelStartDate(trip);
  if (!travel) return false;
  travel.setHours(0, 0, 0, 0);
  const startDay = new Date(start);
  startDay.setHours(0, 0, 0, 0);
  const endDay = new Date(end);
  endDay.setHours(23, 59, 59, 999);
  const ms = travel.getTime();
  return ms >= startDay.getTime() && ms <= endDay.getTime();
}

function comptaApelCommitmentForTrip(
  trip: TravelsTrip,
  sheetOverride?: TravelsComptaSheet | null,
): ComptaApelTripCommitment {
  const sheet = sheetOverride
    ? computeComptaSheetDerived(sheetOverride)
    : readComptaSheetFromTrip(trip);
  const { apelCollective, aidesIndividuelles, totalApel } = apelCommitmentFromSheet(sheet);
  const travelRaw = trip.data?.startDate || trip.data?.date || null;
  return {
    tripId: trip.id,
    title: String(trip.data?.title || "Sortie sans titre"),
    etablissement: normalizeApelEtablissement(trip.data?.etablissement),
    travelDate: travelRaw ? String(travelRaw) : null,
    travelDateLabel: formatTravelDateLabel(trip),
    apelCollective,
    aidesIndividuelles,
    totalApel,
    hasComptaSheet: sheet != null,
  };
}

export function buildComptaApelSummary(
  trips: TravelsTrip[],
  options?: { currentTripId?: string | null; currentSheet?: TravelsComptaSheet | null; now?: Date },
): ComptaApelSummary {
  const bounds = comptaApelSchoolYearBounds(options?.now);
  const currentTripId = options?.currentTripId ?? null;
  const rows: ComptaApelTripCommitment[] = [];

  for (const trip of trips) {
    if (!tripInApelSchoolYear(trip, bounds.start, bounds.end)) continue;
    const sheetOverride = trip.id === currentTripId ? options?.currentSheet : undefined;
    const row = comptaApelCommitmentForTrip(trip, sheetOverride);
    if (row.totalApel <= 0) continue;
    rows.push(row);
  }

  rows.sort((a, b) => {
    const da = a.travelDate ? parseTripLocalDate(a.travelDate)?.getTime() ?? 0 : 0;
    const db = b.travelDate ? parseTripLocalDate(b.travelDate)?.getTime() ?? 0 : 0;
    return da - db;
  });

  const totals = computeApelTotals(rows);
  const byEtablissement = buildApelGroupsByEtablissement(rows);

  const currentTrip = currentTripId ? rows.find((r) => r.tripId === currentTripId) ?? null : null;

  return {
    schoolYear: {
      label: bounds.label,
      startIso: bounds.startIso,
      endIso: bounds.endIso,
    },
    currentTripId,
    currentTrip,
    byEtablissement,
    trips: rows,
    totals,
  };
}
