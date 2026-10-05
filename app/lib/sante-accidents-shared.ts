/** Types registre accidents — safe client & serveur. */

export type SanteAccidentRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateAccident: string;
  circonstances: string;
  soins: string;
  suite: string;
  lieu: string | null;
  auteurNom: string | null;
};
