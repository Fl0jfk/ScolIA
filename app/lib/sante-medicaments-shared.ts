/** Types prises médicament — safe client & serveur. */

export type SanteMedicamentPriseRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  medicament: string;
  dose: string | null;
  prisAt: string;
  auteurNom: string | null;
  notes: string | null;
};
