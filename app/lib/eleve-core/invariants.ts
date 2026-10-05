/** Invariants du socle élève — purs, sans I/O. Décisions Florian 21 sept 2026. */

import { canonicalRegimeLabel, classifyRegime } from "@/app/lib/eleve-regime";

export const SCOLARITE_STATUTS = ["en_cours", "prevue", "terminee", "annulee"] as const;
export type ScolariteStatut = (typeof SCOLARITE_STATUTS)[number];

export class EleveCoreError extends Error {
  readonly code: string;
  constructor(code: string, message: string) {
    super(message);
    this.name = "EleveCoreError";
    this.code = code;
  }
}

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

export function isIsoDate(raw: string): boolean {
  if (!ISO_DATE.test(raw)) return false;
  const t = Date.parse(`${raw}T00:00:00Z`);
  if (Number.isNaN(t)) return false;
  const d = new Date(t);
  const yyyy = d.getUTCFullYear();
  const mm = String(d.getUTCMonth() + 1).padStart(2, "0");
  const dd = String(d.getUTCDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}` === raw;
}

export function assertIsoDate(raw: string, label = "date"): string {
  const v = raw.trim();
  if (!isIsoDate(v)) {
    throw new EleveCoreError("INVALID_DATE", `${label} invalide (AAAA-MM-JJ).`);
  }
  return v;
}

/** Jour calendaire précédent (UTC, dates métier AAAA-MM-JJ). */
export function dayBefore(iso: string): string {
  const v = assertIsoDate(iso);
  const d = new Date(`${v}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() - 1);
  const yyyy = d.getUTCFullYear();
  const mm = String(d.getUTCMonth() + 1).padStart(2, "0");
  const dd = String(d.getUTCDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}`;
}

export function compareIsoDate(a: string, b: string): number {
  return assertIsoDate(a).localeCompare(assertIsoDate(b));
}

/** Aligné sur `ent-core-db.currentSchoolYearLabel` (août = nouvelle année). */
export function currentSchoolYearLabel(now = new Date()): string {
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 7) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}

export function nextSchoolYearLabelFrom(label: string): string {
  const m = /^(\d{4})-(\d{4})$/.exec(label.trim());
  if (!m) {
    throw new EleveCoreError("INVALID_YEAR", `Libellé d’année invalide : ${label}`);
  }
  const y0 = Number(m[1]);
  const y1 = Number(m[2]);
  return `${y0 + 1}-${y1 + 1}`;
}

export function parseScolariteStatut(raw: unknown): ScolariteStatut | undefined {
  const v = String(raw ?? "")
    .trim()
    .toLowerCase();
  if (v === "en_cours" || v === "prevue" || v === "terminee" || v === "annulee") return v;
  return undefined;
}

export function storedRegimeLabel(raw: string | null | undefined): string | null {
  const trimmed = String(raw ?? "").trim();
  if (!trimmed) return null;
  return canonicalRegimeLabel(trimmed) ?? trimmed;
}

export function regimeImpliesDemiPension(raw: string | null | undefined): boolean {
  return classifyRegime(raw) === "demi_pension";
}

export type RegimePeriodInput = {
  regime: string;
  dateDebut: string;
  dateFin: string | null;
};

/**
 * Ferme la période ouverte la veille de `effectiveOn`, ouvre la nouvelle.
 * Même régime déjà ouvert → no-op.
 * Même jour que le début de la période ouverte → correction in-place (`replaceOpen`).
 */
export function planRegimeCutover(args: {
  open: RegimePeriodInput | null;
  nextRegime: string;
  effectiveOn: string;
}): {
  noop: boolean;
  replaceOpen: boolean;
  closeDateFin: string | null;
  next: RegimePeriodInput;
} {
  const effectiveOn = assertIsoDate(args.effectiveOn, "date d’effet");
  const nextRegime = storedRegimeLabel(args.nextRegime);
  if (!nextRegime) {
    throw new EleveCoreError("INVALID_REGIME", "Régime requis.");
  }

  if (!args.open) {
    return {
      noop: false,
      replaceOpen: false,
      closeDateFin: null,
      next: { regime: nextRegime, dateDebut: effectiveOn, dateFin: null },
    };
  }

  const openRegime = storedRegimeLabel(args.open.regime) ?? args.open.regime;
  if (openRegime === nextRegime && args.open.dateFin == null) {
    return {
      noop: true,
      replaceOpen: false,
      closeDateFin: null,
      next: { ...args.open, regime: openRegime },
    };
  }

  const cmp = compareIsoDate(effectiveOn, args.open.dateDebut);
  if (cmp < 0) {
    throw new EleveCoreError(
      "REGIME_DATE_ORDER",
      "La date d’effet doit être après (ou égale à) le début de la période actuelle.",
    );
  }
  if (cmp === 0) {
    return {
      noop: false,
      replaceOpen: true,
      closeDateFin: null,
      next: { regime: nextRegime, dateDebut: effectiveOn, dateFin: null },
    };
  }

  const closeDateFin = dayBefore(effectiveOn);
  if (compareIsoDate(closeDateFin, args.open.dateDebut) < 0) {
    throw new EleveCoreError(
      "REGIME_DATE_ORDER",
      "La période actuelle ne peut pas se terminer avant son début.",
    );
  }

  return {
    noop: false,
    replaceOpen: false,
    closeDateFin,
    next: { regime: nextRegime, dateDebut: effectiveOn, dateFin: null },
  };
}

/** Régime en vigueur à une date (pour facturation). Fin inclusive. */
export function regimeAtDate(
  periods: RegimePeriodInput[],
  on: string,
): string | null {
  const day = assertIsoDate(on, "date");
  const covering = periods.filter((p) => {
    if (compareIsoDate(day, p.dateDebut) < 0) return false;
    if (p.dateFin == null) return true;
    return compareIsoDate(day, p.dateFin) <= 0;
  });
  covering.sort((a, b) => compareIsoDate(b.dateDebut, a.dateDebut));
  return covering[0] ? storedRegimeLabel(covering[0].regime) ?? covering[0].regime : null;
}

export function assertDifferentYears(currentAnneeId: string | null, nextAnneeId: string): void {
  if (currentAnneeId && currentAnneeId === nextAnneeId) {
    throw new EleveCoreError(
      "SAME_YEAR",
      "La classe prévue doit porter sur une autre année scolaire, pas la même.",
    );
  }
}

export function assertNotTwoClassesSameYear(opts: {
  existingActiveSameYearId: string | null;
  incomingId?: string | null;
}): void {
  if (!opts.existingActiveSameYearId) return;
  if (opts.incomingId && opts.existingActiveSameYearId === opts.incomingId) return;
  throw new EleveCoreError(
    "TWO_CLASSES_SAME_YEAR",
    "Un élève ne peut pas être dans deux classes sur une même année scolaire.",
  );
}
