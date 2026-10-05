/** Constantes & types extraits santé diffusés — safe client & serveur. */

export const SANTE_EXTRAIT_PORTEES = [
  "cantine",
  "eps",
  "voyage",
  "internat",
  "periscolaire",
] as const;

export type SanteExtraitPortee = (typeof SANTE_EXTRAIT_PORTEES)[number];

export const SANTE_EXTRAIT_PORTEE_LABELS: Record<SanteExtraitPortee, string> = {
  cantine: "Cantine",
  eps: "EPS",
  voyage: "Voyage",
  internat: "Internat",
  periscolaire: "Périscolaire",
};

export type SanteExtraitRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  portee: SanteExtraitPortee;
  libelle: string;
  actif: boolean;
  documentId: string | null;
};

export function isSanteExtraitPortee(v: string): v is SanteExtraitPortee {
  return (SANTE_EXTRAIT_PORTEES as readonly string[]).includes(v);
}
