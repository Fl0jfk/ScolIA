/**
 * Lissage mensuel du prix d'un voyage sur les factures habituelles (établissement privé) — fonctions pures.
 * Calcul en centimes : la somme des mensualités est exacte au centime, le reste d'arrondi va sur le dernier mois.
 * Le montant est un paramètre : la fonction pourra être rappelée plus tard sur le reste à charge d'une famille aidée.
 */

/** Mois (1-12) proposé par défaut pour le dernier mois facturé ; la valeur exacte (juin ou juillet) reste à confirmer. */
export const DEFAULT_LAST_BILLING_MONTH_NUMBER = 7;

/** Garde-fou : au-delà, la saisie est considérée comme une erreur. */
export const MAX_BILLING_MONTHS = 24;

const MONTH_RE = /^(\d{4})-(0[1-9]|1[0-2])$/;

export function isYearMonth(s: unknown): s is string {
  return typeof s === "string" && MONTH_RE.test(s);
}

/** Dernier mois facturé proposé : DEFAULT_LAST_BILLING_MONTH_NUMBER de l'année scolaire en cours (YYYY-MM). */
export function defaultLastBillingMonth(todayIso: string): string {
  const y = Number(todayIso.slice(0, 4));
  const m = Number(todayIso.slice(5, 7));
  const schoolYearEnd = m >= 8 ? y + 1 : y;
  return `${schoolYearEnd}-${String(DEFAULT_LAST_BILLING_MONTH_NUMBER).padStart(2, "0")}`;
}

const MONTHS_FR = [
  "janvier", "février", "mars", "avril", "mai", "juin",
  "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

export function monthLabelFr(ym: string): string {
  const m = MONTH_RE.exec(ym);
  return m ? `${MONTHS_FR[Number(m[2]) - 1]} ${m[1]}` : ym;
}

function monthIndex(ym: string): number {
  const m = MONTH_RE.exec(ym)!;
  return Number(m[1]) * 12 + Number(m[2]) - 1;
}

function indexToMonth(i: number): string {
  return `${Math.floor(i / 12)}-${String((i % 12) + 1).padStart(2, "0")}`;
}

export type BillingMonth = { month: string; label: string; amount: number };

export type BillingSchedule =
  | {
      ok: true;
      /** Vide si le montant est à 0 € (pas de lissage). */
      months: BillingMonth[];
      count: number;
      monthlyAmount: number;
      lastAmount: number;
      /** Le dernier mois diffère des autres à cause de l'arrondi. */
      lastDiffers: boolean;
      total: number;
    }
  | { ok: false; error: string };

/**
 * Répartit `amount` (euros) du premier au dernier mois inclus.
 * Montant mensuel = montant / nombre de mois arrondi au centime ; le reste va sur le dernier mois.
 */
export function computeMonthlySchedule(
  amount: number,
  firstMonth: string | null | undefined,
  lastMonth: string | null | undefined,
): BillingSchedule {
  const euros = Number(amount);
  if (!Number.isFinite(euros) || euros < 0) return { ok: false, error: "Montant à répartir invalide." };
  const totalCents = Math.round(euros * 100);
  if (totalCents === 0) {
    return { ok: true, months: [], count: 0, monthlyAmount: 0, lastAmount: 0, lastDiffers: false, total: 0 };
  }
  if (!isYearMonth(firstMonth)) return { ok: false, error: "Indiquez le premier mois facturé (mois et année)." };
  if (!isYearMonth(lastMonth)) return { ok: false, error: "Indiquez le dernier mois facturé (mois et année)." };
  const a = monthIndex(firstMonth);
  const b = monthIndex(lastMonth);
  if (a > b) {
    return {
      ok: false,
      error: `Le premier mois facturé (${monthLabelFr(firstMonth)}) est après le dernier (${monthLabelFr(lastMonth)}).`,
    };
  }
  const count = b - a + 1;
  if (count > MAX_BILLING_MONTHS) {
    return { ok: false, error: `Période trop longue (${MAX_BILLING_MONTHS} mois maximum).` };
  }
  let monthlyCents = Math.round(totalCents / count);
  if (monthlyCents * (count - 1) > totalCents) monthlyCents = Math.floor(totalCents / count);
  const lastCents = totalCents - monthlyCents * (count - 1);
  const months: BillingMonth[] = [];
  for (let i = 0; i < count; i++) {
    const month = indexToMonth(a + i);
    months.push({
      month,
      label: monthLabelFr(month),
      amount: (i === count - 1 ? lastCents : monthlyCents) / 100,
    });
  }
  return {
    ok: true,
    months,
    count,
    monthlyAmount: monthlyCents / 100,
    lastAmount: lastCents / 100,
    lastDiffers: lastCents !== monthlyCents,
    total: totalCents / 100,
  };
}
