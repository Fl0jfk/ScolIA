export type DomainPlanningDomain = {
  id: string;
  name: string;
  description?: string;
  color?: string;
  coordinatorExternalUserIds: string[];
};

/** @deprecated Ancien modèle « réservation de créneau » — conservé pour compatibilité données. */
export type DomainPlanningBooking = {
  id: string;
  groupId?: string | null;
  domainId: string;
  userId: string;
  firstName: string;
  lastName: string;
  email: string;
  className: string;
  activityLabel?: string;
  comment: string;
  startsAt: string;
  endsAt: string;
  createdAt: string;
  createdByUserId: string;
  assignmentKind: "coordinator" | "self";
  status: "CONFIRMED" | "CANCELLED";
  cancelledAt?: string;
  cancelledBy?: string;
  cancelReason?: string;
};

export type TransversalNiveau = "6e" | "5e" | "4e" | "3e";

export type TransversalIntervenantConstraint =
  | "svt_only"
  | "fixed_association"
  | "psy_inf"
  | "free";

export type DomainPlanningSession = {
  id: string;
  niveau: TransversalNiveau;
  seanceNumber: 1 | 2 | 3;
  theme: string;
  /** Libellé affiché (ex. « Profs d'SVT », « Association »). */
  intervenantLabel: string;
  intervenantConstraint: TransversalIntervenantConstraint;
  mixte: boolean;
};

export type DomainPlanningSignupValidationStatus =
  | "pending"
  | "validated"
  | "changes_requested"
  | "rejected";

export type DomainPlanningSignup = {
  id: string;
  sessionId: string;
  className: string;
  userId: string;
  firstName: string;
  lastName: string;
  email: string;
  subject: string;
  sessionIdea?: string;
  createdAt: string;
  validationStatus?: DomainPlanningSignupValidationStatus;
  validatedAt?: string;
  validatedByUserId?: string;
  /** Commentaire de la responsable EVARS (modifications demandées ou refus). */
  validationComment?: string;
};
