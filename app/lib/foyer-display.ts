/**
 * Affichage foyer-first (facturation, portail, dossier élève).
 * Évite la logique Charlemagne « le père paie / signe seul ».
 */

export type FoyerResponsableDisplay = {
  prenom: string;
  nom: string;
  email?: string | null;
  autoriteParentale?: boolean;
  contactUrgence?: boolean;
  payeur?: boolean;
  userId?: string | null;
};

export type FoyerBillingDisplayInput = {
  label: string;
  payeurEstFoyer: boolean;
  responsables: FoyerResponsableDisplay[];
};

/** Libellé principal facturation / signature. */
export function formatFoyerFacturationLabel(foyer: FoyerBillingDisplayInput): string {
  return foyer.label.trim() || "Foyer";
}

/** Sous-titre payeur : toujours foyer quand payeurEstFoyer. */
export function formatFoyerPayeurDetail(foyer: FoyerBillingDisplayInput): string {
  if (foyer.payeurEstFoyer) {
    const withAuthority = foyer.responsables.filter((r) => r.autoriteParentale !== false);
    if (withAuthority.length >= 2) {
      return "Facturation et documents au nom du foyer — chaque responsable avec autorité parentale peut agir avec son propre compte.";
    }
    return "Facturation au foyer — pas rattachée à un seul parent.";
  }
  const payeurs = foyer.responsables.filter((r) => r.payeur);
  if (payeurs.length === 1) {
    return `Responsable payeur : ${payeurs[0]!.prenom} ${payeurs[0]!.nom}`;
  }
  if (payeurs.length > 1) {
    return payeurs.map((r) => `${r.prenom} ${r.nom}`).join(", ");
  }
  return "Responsable payeur à désigner";
}

/** Tags rôle responsable (sans « payeur » si facturation = foyer). */
export function responsableRoleTags(
  r: FoyerResponsableDisplay,
  payeurEstFoyer: boolean,
): string[] {
  const tags: string[] = [];
  if (r.autoriteParentale) tags.push("autorité parentale");
  if (r.contactUrgence) tags.push("urgence");
  if (!payeurEstFoyer && r.payeur) tags.push("payeur");
  if (r.userId) tags.push("compte connecté");
  return tags;
}

export type EncoursAnneeRow = {
  anneeScolaireId: string | null;
  anneeLabel: string | null;
  montantRestant: number;
  factureCount: number;
};

export function formatEncoursMontant(montant: number): string {
  if (montant <= 0.009) return "0 €";
  return `${montant.toFixed(2).replace(".", ",")} €`;
}
