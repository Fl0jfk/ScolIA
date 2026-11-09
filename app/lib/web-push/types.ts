/** Contrat partagé des notifications navigateur (Web Push). */

/** Catégorie de notification — sert aux préférences (« couper les notifications de … »). */
export type WebPushKind =
  | "messagerie"
  | "messagerie_mention"
  | "signal"; // compteurs du tableau de bord (demandes, absences, sorties, …)

/**
 * Contenu affiché par le navigateur. Minimal par conception : il transite (chiffré)
 * par les serveurs de Google / Apple / Mozilla. Jamais de données sensibles d'élève.
 */
export type WebPushPayload = {
  kind: WebPushKind;
  title: string;
  body: string;
  /** Chemin interne ouvert au clic (ex. « /messagerie?c=<id> »). Toujours relatif. */
  url: string;
  /** Regroupe / remplace les notifications de même tag (ex. une par conversation). */
  tag?: string;
};

export type WebPushSendInput = {
  etablissementId: string;
  userIds: string[];
  payload: WebPushPayload;
};
