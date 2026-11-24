/**
 * Annulation d'un devis de transport déjà signé : possible jusqu'à 14 jours avant le départ
 * (inclus), puis verrouillée.
 */
export const BUS_QUOTE_CANCEL_MIN_DAYS_BEFORE = 14;

/** Statuts où un devis signé peut exister et être annulé pour en choisir un autre. */
export const BUS_QUOTE_CANCEL_ALLOWED_STATUSES = [
  "EN_ATTENTE_COMPTA",
  "EN_ATTENTE_DIR_FINAL",
  "FINALISE_DIR_ATTENTE_ELEVES",
  "VALIDE",
  "EN_ATTENTE_BUS_SIGNATURE",
] as const;

function dayStartUtc(isoOrDate: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(String(isoOrDate || "").trim());
  if (!m) return null;
  return Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
}

function todayParisUtc(now: Date): number {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now);
  return dayStartUtc(parts) ?? Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
}

export type BusQuoteCancelWindow = {
  /** Dernier jour (AAAA-MM-JJ) où l'annulation est encore possible, ou null si date de départ inconnue. */
  lastDay: string | null;
  open: boolean;
  reason: string | null;
};

export function busQuoteCancelWindow(
  startDate: string | null | undefined,
  now: Date = new Date(),
): BusQuoteCancelWindow {
  const start = dayStartUtc(String(startDate || ""));
  if (start === null) {
    return {
      lastDay: null,
      open: false,
      reason: "Date de départ inconnue : annulation impossible.",
    };
  }
  const last = start - BUS_QUOTE_CANCEL_MIN_DAYS_BEFORE * 86_400_000;
  const lastDay = new Date(last).toISOString().slice(0, 10);
  if (todayParisUtc(now) > last) {
    return {
      lastDay,
      open: false,
      reason: `Verrouillé : l'annulation n'est possible que jusqu'à ${BUS_QUOTE_CANCEL_MIN_DAYS_BEFORE} jours avant le départ (jusqu'au ${lastDay}).`,
    };
  }
  return { lastDay, open: true, reason: null };
}
