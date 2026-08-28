import "server-only";

import {
  getCurrentAnneeScolaire,
  upsertAnneeScolaire,
  type AnneeScolaireRow,
} from "@/app/lib/annees-scolaires-db";
import {
  countEncoursFacturationEtab,
  resetQuotientCategoriesForEtab,
} from "@/app/lib/facturation-db";

function nextYearLabelFromCurrent(label: string): string {
  const m = /^(\d{4})-(\d{4})$/.exec(label.trim());
  if (!m) throw new Error("Libellé année courante invalide.");
  const y0 = Number(m[1]);
  return `${y0 + 1}-${y0 + 2}`;
}

export type PassageAnneeResult = {
  previousAnnee: AnneeScolaireRow | null;
  newAnnee: AnneeScolaireRow;
  foyersQuotientReset: number;
  encoursFacturesConservees: number;
  message: string;
};

/**
 * Bascule l’année scolaire courante sans « changer de base » :
 * - flip isCurrent + dates
 * - remise à zéro quotient / catégorie facturation uniquement
 * - IBAN, RUM, mandats, factures et encours N-1 intacts
 */
export async function executePassageAnneeScolaire(
  etablissementId: string,
  opts?: { label?: string },
): Promise<PassageAnneeResult> {
  const current = await getCurrentAnneeScolaire(etablissementId);
  const targetLabel = opts?.label?.trim() || (current ? nextYearLabelFromCurrent(current.label) : null);
  if (!targetLabel) {
    throw new Error("Aucune année courante — indiquez le libellé de la nouvelle année.");
  }

  const newAnnee = await upsertAnneeScolaire(etablissementId, {
    label: targetLabel,
    makeCurrent: true,
  });

  const foyersQuotientReset = await resetQuotientCategoriesForEtab(etablissementId);
  const encoursFacturesConservees = await countEncoursFacturationEtab(etablissementId);

  const message =
    `Année ${newAnnee.label} activée. ` +
    `${foyersQuotientReset} foyer(s) : catégories quotient remises à zéro (IBAN et SEPA conservés). ` +
    `${encoursFacturesConservees} facture(s) avec reste à payer conservée(s) — historique intact.`;

  return {
    previousAnnee: current,
    newAnnee,
    foyersQuotientReset,
    encoursFacturesConservees,
    message,
  };
}

/** Libellé année courante pour affichage discret (header, portail). */
export async function getAnneeCouranteLabel(etablissementId: string): Promise<string | null> {
  const current = await getCurrentAnneeScolaire(etablissementId);
  return current?.label ?? null;
}
