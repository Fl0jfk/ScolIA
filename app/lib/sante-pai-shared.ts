/** Constantes & types PAI — safe client & serveur. */

export const SANTE_PAI_STATUTS = ["brouillon", "valide", "expire", "revoque"] as const;

export type SantePaiStatut = (typeof SANTE_PAI_STATUTS)[number];

export const SANTE_PAI_STATUT_LABELS: Record<SantePaiStatut, string> = {
  brouillon: "Brouillon",
  valide: "Validé",
  expire: "Expiré",
  revoque: "Révoqué",
};

export type SantePaiRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  statut: SantePaiStatut;
  protocole: string;
  traitementsAutorises: string;
  documentId: string | null;
  documentTitle: string | null;
  dateDebut: string | null;
  dateFin: string | null;
  valideAt: string | null;
  valideParNom: string | null;
  notes: string;
  updatedAt: string;
};

export type SantePaiDocumentLite = {
  id: string;
  title: string;
  createdAt: string;
};

export function isSantePaiStatut(v: string): v is SantePaiStatut {
  return (SANTE_PAI_STATUTS as readonly string[]).includes(v);
}
