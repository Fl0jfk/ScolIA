/** Constantes & types passages établissement — safe client & serveur. */

export const PASSAGE_SENS = ["entree", "sortie"] as const;
export type PassageSens = (typeof PASSAGE_SENS)[number];

export const PASSAGE_SENS_LABELS: Record<PassageSens, string> = {
  entree: "Entrée",
  sortie: "Sortie",
};

export const PASSAGE_LIEUX = ["portail", "self", "internat", "infirmerie", "autre"] as const;
export type PassageLieu = (typeof PASSAGE_LIEUX)[number];

export const PASSAGE_LIEU_LABELS: Record<PassageLieu, string> = {
  portail: "Portail",
  self: "Self (repas)",
  internat: "Internat",
  infirmerie: "Infirmerie",
  autre: "Autre",
};

export type PassageRow = {
  id: string;
  eleveId: string | null;
  eleveNom: string | null;
  elevePrenom: string | null;
  eleveClasse: string | null;
  inviteNom: string | null;
  sens: PassageSens;
  lieu: PassageLieu;
  horodatage: string;
  source: string;
};

export function isPassageSens(v: string): v is PassageSens {
  return (PASSAGE_SENS as readonly string[]).includes(v);
}

export function isPassageLieu(v: string): v is PassageLieu {
  return (PASSAGE_LIEUX as readonly string[]).includes(v);
}
