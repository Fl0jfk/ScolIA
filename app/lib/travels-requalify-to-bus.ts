import type { TravelsTripData } from "@/app/lib/travels-types";

/** Statuts déjà au-delà de la pédagogie : on ramène au choix de devis. */
export const REQUALIFY_STATUSES_NEEDING_LOGISTICS = new Set([
  "PROF_LOGISTICS",
  "EN_ATTENTE_BUS_SIGNATURE",
  "EN_ATTENTE_COMPTA",
  "EN_ATTENTE_DIR_FINAL",
  "VALIDE",
]);

export function resolveRequalifyToBusStatus(
  current: string,
  previousStatus?: string,
): { status: string; previousStatus?: string } {
  if (current === "EN_ATTENTE_DIR_INITIAL") {
    return { status: current };
  }
  if (current === "BESOIN_MODIFICATION") {
    const prev = previousStatus || "";
    if (REQUALIFY_STATUSES_NEEDING_LOGISTICS.has(prev) || prev === "EN_ATTENTE_COMPTA") {
      return { status: current, previousStatus: "PROF_LOGISTICS" };
    }
    return { status: current, previousStatus: previousStatus || undefined };
  }
  if (REQUALIFY_STATUSES_NEEDING_LOGISTICS.has(current)) {
    return { status: "PROF_LOGISTICS" };
  }
  return { status: current };
}

export function normalizeDatesForComplexTrip(data: TravelsTripData): TravelsTripData {
  const day = String(data.startDate || data.date || "").trim();
  const startDate = String(data.startDate || day).trim() || day;
  const endDate = String(data.endDate || startDate || day).trim() || startDate;
  return {
    ...data,
    date: data.date || startDate || undefined,
    startDate: startDate || undefined,
    endDate: endDate || undefined,
  };
}

export function mergeTransportRequestForRequalify(
  existing: TravelsTripData["transportRequest"],
  incoming?: {
    pickupPoint?: string;
    stayOnSite?: boolean;
    freeText?: string;
  },
): Record<string, unknown> {
  const base =
    existing && typeof existing === "object" && !Array.isArray(existing)
      ? { ...existing }
      : {};
  if (!incoming) return base;
  if (typeof incoming.pickupPoint === "string") {
    base.pickupPoint = incoming.pickupPoint.trim();
  }
  if (typeof incoming.stayOnSite === "boolean") {
    base.stayOnSite = incoming.stayOnSite;
  }
  if (typeof incoming.freeText === "string") {
    base.freeText = incoming.freeText.trim();
  }
  return base;
}
