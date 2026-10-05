/** Appel du soir internat (Postgres) — types safe client. */

export const INTERNAT_APPEL_MARQUES = ["present", "absent", "excuse", "activite"] as const;
export type InternatAppelMarque = (typeof INTERNAT_APPEL_MARQUES)[number];

export const INTERNAT_APPEL_MARQUE_LABELS: Record<InternatAppelMarque, string> = {
  present: "Présent",
  absent: "Absent",
  excuse: "Excusé",
  activite: "Activité",
};

export const INTERNAT_APPEL_STATUTS = ["ouverte", "validee"] as const;
export type InternatAppelStatut = (typeof INTERNAT_APPEL_STATUTS)[number];

export function isInternatAppelMarque(v: string): v is InternatAppelMarque {
  return (INTERNAT_APPEL_MARQUES as readonly string[]).includes(v);
}

export type InternatAppelLigneRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  chambreLabel: string | null;
  batimentLabel: string | null;
  marque: InternatAppelMarque;
  note: string | null;
};

export type InternatAppelResume = {
  id: string;
  dateAppel: string;
  statut: InternatAppelStatut;
  batimentId: string | null;
  valideAt: string | null;
  total: number;
  presents: number;
  absents: number;
  excuses: number;
  activites: number;
};

export type InternatAppelSoirPayload = {
  appel: InternatAppelResume;
  lignes: InternatAppelLigneRow[];
};
