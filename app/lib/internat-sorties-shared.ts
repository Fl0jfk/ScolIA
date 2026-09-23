/** Sorties week-end internat (Postgres) — types safe client. */

export type InternatSortieRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateDebut: string;
  dateFin: string;
  motif: string | null;
};
