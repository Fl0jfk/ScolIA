/** Noms d’événements — uniquement s’il existe déjà un 2ᵉ consommateur. */

export const METIER_EVENT_TYPES = {
  ELEVE_CREATED: "eleve.created",
  ELEVE_STATUS_CHANGED: "eleve.status_changed",
  ELEVE_IDENTITY_CHANGED: "eleve.identity_changed",
  SCOLARITE_OPENED: "scolarite.opened",
  SCOLARITE_CLASSE_CHANGED: "scolarite.classe_changed",
  SCOLARITE_CLOSED: "scolarite.closed",
  ELEVE_REGIME_CHANGED: "eleve.regime_changed",
  SCOLARITE_GRILLE_REPAS_CHANGED: "scolarite.grille_repas_changed",
  FOYER_CHANGED: "foyer.changed",
} as const;

export type MetierEventType = (typeof METIER_EVENT_TYPES)[keyof typeof METIER_EVENT_TYPES];

export type MetierAggregate = "eleve" | "scolarite" | "foyer" | "regime";

export type MetierEventRecord = {
  etablissementId: string;
  type: MetierEventType | string;
  aggregate: MetierAggregate;
  aggregateId: string;
  eleveId?: string | null;
  payload?: Record<string, unknown>;
  actorUserId?: string | null;
};
