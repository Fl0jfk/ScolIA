/** Types internat chambres / affectations — safe client. */

export type InternatBatimentRow = {
  id: string;
  label: string;
  notes: string | null;
};

export type InternatChambreRow = {
  id: string;
  batimentId: string;
  batimentLabel: string;
  label: string;
  etage: string | null;
  capacite: number;
  aile: string | null;
  placesOccupees: number;
};

export type InternatAffectationRow = {
  id: string;
  chambreId: string;
  chambreLabel: string;
  batimentLabel: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateDebut: string;
  dateFin: string | null;
};
