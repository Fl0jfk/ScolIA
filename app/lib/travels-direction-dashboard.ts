export type TripDashboardRow = {
  id: string;
  type?: string;
  status?: string;
  ownerName?: string;
  data?: {
    title?: string;
    etablissement?: string;
    date?: string;
    startDate?: string;
    endDate?: string;
    needsBus?: boolean;
    selectedBusQuote?: unknown;
  };
  createdAt?: string;
};

export type TravelsDirectionDashboard = {
  etablissement: string;
  schoolYearLabel: string;
  counts: {
    signaturesPending: number;
    tripsYear: number;
    tripsActive: number;
    pendingPedagogy: number;
    pendingBusChoice: number;
    upcomingTrips: number;
    validatedYear: number;
    busQuotesWaiting: number;
  };
  signaturesPending: Array<{ id: string; title: string; status: string; reason: string }>;
  upcoming: Array<{ id: string; title: string; dateLabel: string; status: string }>;
};

const ACTIVE_STATUSES = new Set([
  "EN_ATTENTE_DIR_INITIAL",
  "EN_ATTENTE_BUS_SIGNATURE",
  "PROF_LOGISTICS",
  "EN_ATTENTE_COMPTA",
  "EN_ATTENTE_DIR_FINAL",
  "BESOIN_MODIFICATION",
]);

function schoolYearBounds(now = new Date()) {
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 8) {
    return {
      start: new Date(y, 8, 1),
      end: new Date(y + 1, 7, 31, 23, 59, 59),
      label: `${y} – ${y + 1}`,
    };
  }
  return {
    start: new Date(y - 1, 8, 1),
    end: new Date(y, 7, 31, 23, 59, 59),
    label: `${y - 1} – ${y}`,
  };
}

function parseTripStart(trip: TripDashboardRow): Date | null {
  const raw = trip.data?.startDate || trip.data?.date;
  if (!raw) return null;
  const d = new Date(raw);
  return Number.isNaN(d.getTime()) ? null : d;
}

function inSchoolYear(trip: TripDashboardRow, start: Date, end: Date) {
  const created = trip.createdAt ? new Date(trip.createdAt) : parseTripStart(trip);
  if (!created || Number.isNaN(created.getTime())) return false;
  return created >= start && created <= end;
}

function formatDateLabel(trip: TripDashboardRow) {
  const start = parseTripStart(trip);
  if (!start) return "Date à préciser";
  if (trip.type === "COMPLEX" && trip.data?.endDate) {
    const end = new Date(trip.data.endDate);
    if (!Number.isNaN(end.getTime())) {
      return `${start.toLocaleDateString("fr-FR")} → ${end.toLocaleDateString("fr-FR")}`;
    }
  }
  return start.toLocaleDateString("fr-FR");
}

import type { Establishment } from "@/app/lib/app-config-schemas";
import { directionRolesMatchEstablishmentRef } from "@/app/lib/establishment-catalog";

export function resolveDirectionEtab(
  roles: string[],
  establishments: Establishment[] = [],
): string | null {
  for (const est of establishments) {
    if (est.active === false) continue;
    if (directionRolesMatchEstablishmentRef(roles, est.label, establishments)) {
      return est.label;
    }
  }
  return null;
}

export function buildTravelsDirectionDashboard(
  trips: TripDashboardRow[],
  etablissement: string,
): TravelsDirectionDashboard {
  const sy = schoolYearBounds();
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const mine = trips.filter((t) => (t.data?.etablissement || "Groupe Scolaire") === etablissement);

  const signaturesPending: TravelsDirectionDashboard["signaturesPending"] = [];
  let pendingPedagogy = 0;
  let pendingBusChoice = 0;
  let busQuotesWaiting = 0;
  let tripsActive = 0;
  let validatedYear = 0;
  let tripsYear = 0;
  const upcoming: TravelsDirectionDashboard["upcoming"] = [];

  for (const t of mine) {
    if (t.status === "SEANCE_ANNULEE" || t.status === "REJETE") continue;

    if (inSchoolYear(t, sy.start, sy.end)) tripsYear += 1;
    if (t.status === "VALIDE" && inSchoolYear(t, sy.start, sy.end)) validatedYear += 1;

    if (t.status && ACTIVE_STATUSES.has(t.status)) tripsActive += 1;

    if (t.status === "EN_ATTENTE_DIR_INITIAL") {
      pendingPedagogy += 1;
      signaturesPending.push({
        id: t.id,
        title: t.data?.title || "Sans titre",
        status: t.status,
        reason: "Validation pédagogique",
      });
    }
    if (t.status === "EN_ATTENTE_BUS_SIGNATURE") {
      signaturesPending.push({
        id: t.id,
        title: t.data?.title || "Sans titre",
        status: t.status,
        reason: "Signature devis bus",
      });
    }
    if (t.status === "EN_ATTENTE_DIR_FINAL") {
      signaturesPending.push({
        id: t.id,
        title: t.data?.title || "Sans titre",
        status: t.status,
        reason: "Validation finale",
      });
    }

    const needsBus = t.type === "COMPLEX" && Boolean(t.data?.needsBus);
    if (needsBus && t.status === "PROF_LOGISTICS" && !t.data?.selectedBusQuote) {
      pendingBusChoice += 1;
      busQuotesWaiting += 1;
    }
    if (needsBus && t.status === "EN_ATTENTE_BUS_SIGNATURE") {
      busQuotesWaiting += 1;
    }

    const start = parseTripStart(t);
    if (start && start >= today && t.status !== "VALIDE" && t.status !== "REJETE") {
      upcoming.push({
        id: t.id,
        title: t.data?.title || "Sans titre",
        dateLabel: formatDateLabel(t),
        status: t.status || "—",
      });
    }
  }

  upcoming.sort((a, b) => a.dateLabel.localeCompare(b.dateLabel, "fr"));

  return {
    etablissement,
    schoolYearLabel: sy.label,
    counts: {
      signaturesPending: signaturesPending.length,
      tripsYear,
      tripsActive,
      pendingPedagogy,
      pendingBusChoice,
      upcomingTrips: upcoming.length,
      validatedYear,
      busQuotesWaiting,
    },
    signaturesPending: signaturesPending.slice(0, 8),
    upcoming: upcoming.slice(0, 6),
  };
}
