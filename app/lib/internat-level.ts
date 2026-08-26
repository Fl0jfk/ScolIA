import { niveauScolaireLabel } from "@/app/lib/pilotage-eleves-logic";

/** Niveaux collège / lycée utiles pour l’appel et le roster internat. */
export const INTERNAT_NIVEAUX = ["6e", "5e", "4e", "3e", "2nde", "1re", "Tle"] as const;

export type InternatNiveau = (typeof INTERNAT_NIVEAUX)[number];

const NIVEAU_ORDER: Record<string, number> = {
  "6e": 1,
  "5e": 2,
  "4e": 3,
  "3e": 4,
  "2nde": 5,
  "1re": 6,
  Tle: 7,
};

/** Extrait le niveau scolaire d’une classe (ex. « 1re A » → « 1re »). */
export function niveauFromClasse(classe: string | undefined | null): InternatNiveau | null {
  const label = niveauScolaireLabel(classe ?? undefined);
  if (!label) return null;
  if ((INTERNAT_NIVEAUX as readonly string[]).includes(label)) {
    return label as InternatNiveau;
  }
  return null;
}

export function niveauSortKey(niveau: string | null | undefined): number {
  if (!niveau) return 99;
  return NIVEAU_ORDER[niveau] ?? 50;
}

export function niveauDisplayLabel(niveau: string | null | undefined): string {
  if (!niveau) return "Autres";
  return niveau;
}
