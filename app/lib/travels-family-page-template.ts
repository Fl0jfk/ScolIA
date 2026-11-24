/**
 * TRAME FIXE de la page des familles (circuit v2) — un seul fichier de texte à ajuster.
 * Le même texte s'applique à tous les voyages ; les blocs automatiques (titre, destination, dates, classes,
 * organisateur, image, prix, échéances, date limite, pièces, rendez-vous, équipement, liquide, programme)
 * viennent du dossier. Les espaces {entre accolades} sont remplacés automatiquement.
 *
 * Variables disponibles : {titre} {destination} {dates} {classes} {organisateur} {etablissement} {dateLimite}
 */

export const FAMILY_PAGE_TEMPLATE = {
  /** Paragraphes d'introduction (en haut de la page). */
  intro: [
    "Chers parents,",
    "Nous avons le plaisir de proposer aux élèves de {classes} un voyage scolaire : « {titre} », à destination de {destination}, {dates}. Il est organisé par {organisateur} ({etablissement}).",
    "Pour que votre enfant puisse participer, merci de répondre sur cette page avant le {dateLimite}. Sans votre signature, votre enfant ne sera pas inscrit.",
  ],

  /** Titres de sections. */
  headings: {
    meeting: "Rendez-vous et horaires",
    outbound: "Aller",
    inbound: "Retour",
    program: "Programme",
    equipment: "Dans la valise",
    cash: "Argent de poche / liquide à prévoir",
    meals: "Repas et paniers",
    documents: "Pièces à fournir",
    price: "Participation financière",
    deadline: "Date limite de réponse",
    help: "Difficultés financières",
    data: "Conservation de vos données",
  },

  /** Mention affichée sous la somme en liquide. */
  cashNotIncluded: "Cette somme n'est pas comprise dans le prix du voyage.",

  /** Règle des paniers repas. */
  meals: [
    "L'établissement fournit un panier repas uniquement les jours indiqués ci-dessous. Pour chacun de ces jours, répondez « oui » et choisissez le panier (classique, sans porc ou végétarien), ou « non » si vous fournissez vous-même le repas de votre enfant.",
    "En cas d'allergie ou de PAI, le repas est obligatoirement fourni par la famille : la cantine ne prépare pas de repas à la demande.",
  ],

  /** Phrase sur les difficultés financières (formulation de l'attaché de gestion, 24/11/2026). */
  financialHelp:
    "En cas de difficulté financière liée au coût du voyage, les familles sont invitées à prendre rendez-vous avec le chef d’établissement afin d’étudier leur situation. Cet échange permettra d’examiner, le cas échéant, les possibilités d’accompagnement qui pourraient être envisagées.",

  /** Phrase sur la conservation des données. */
  dataRetention:
    "Les pièces que vous déposez sont supprimées le lendemain du retour du voyage. Seul l'engagement signé (formulaire et signature) est conservé par la comptabilité.",

  /** Lissage du prix sur les factures mensuelles (variables : {premierMois} {dernierMois} {n} {montant} {dernier}). */
  billing: {
    intro:
      "Ce montant s'ajoute à vos factures habituelles. Il sera réparti sur vos factures de {premierMois} à {dernierMois} : {n} × {montant} €",
    lastDiffers: " (dernier mois : {dernier} €)",
    oneShot: "Vous pouvez aussi régler en une seule fois : adressez-vous à la comptabilité.",
  },

  /** Fin de tous les e-mails envoyés aux familles (expéditeur mailer@, aucune réponse lue). */
  mailNoReply:
    "Cet e-mail est envoyé automatiquement : merci de ne pas y répondre, cette adresse ne reçoit pas les réponses. Pour toute question, contactez l'organisateur du voyage par la messagerie de l'établissement ou par le carnet de correspondance.",

  /** Libellé affiché quand le bloc prix n'est pas encore saisi par la comptabilité (aperçu projet). */
  pricePending: "À renseigner par la comptabilité.",

  /** Suggestions cliquables pour la liste d'équipement. */
  equipmentSuggestions: [
    "Maillot de bain",
    "Serviette",
    "Gourde",
    "Tenue de sport",
    "Chaussures de sport",
    "Casquette ou bonnet",
    "Crème solaire",
    "Vêtement de pluie",
    "Sac à dos",
    "Pique-nique",
    "Sac pour le linge sale",
    "Trousse de toilette",
  ],
} as const;

export const FAMILY_PAGE_PROGRAM_MAX_CHARS = 600;

/** Remplace les {variables} ; une variable inconnue ou vide devient « … ». */
export function fillFamilyTemplate(text: string, vars: Record<string, string | undefined>): string {
  return text.replace(/\{(\w+)\}/g, (_m, key: string) => {
    const v = vars[key];
    return v && v.trim() ? v : "…";
  });
}
