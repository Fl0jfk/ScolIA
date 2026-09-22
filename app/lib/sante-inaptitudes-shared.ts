/** Types inaptitude EPS — safe client & serveur. */

export type SanteInaptitudeEpsRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateDebut: string;
  dateFin: string | null;
  motif: string;
  libelleExtrait: string;
  documentId: string | null;
  extraitId: string | null;
  actif: boolean;
  auteurNom: string | null;
};
