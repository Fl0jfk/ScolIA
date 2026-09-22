/** Types fiche infirmerie — safe client & serveur. */

export type InfirmerieFicheRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  antecedents: string;
  personnesAPrevenir: string;
  notes: string;
  updatedAt: string;
};
