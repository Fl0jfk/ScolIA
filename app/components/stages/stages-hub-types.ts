import type { PendingStageSignature } from "@/app/lib/stage-pending-signatures";

export type StageTab = "board" | "classe" | "repas" | "settings";

export type StagesHubPermissions = {
  canModerateOffers: boolean;
  canReviewPreconvention: boolean;
  /** File « dépôts à valider » — secrétariat (pas direction seule). */
  canSeeAdminDepositQueue: boolean;
  canViewAllConventions: boolean;
  canViewReferentConventions: boolean;
  canDepositOffer: boolean;
  canFileToOneDrive: boolean;
  canManageStageSettings: boolean;
  canManageReferents: boolean;
  canViewRepasAbsences?: boolean;
  referentOnly: boolean;
  watcherOnly?: boolean;
  canViewClassRoster: boolean;
};

export type StagesHubBoardCard = {
  id: string;
  student?: { firstName: string; lastName: string };
  company?: { name: string };
  studentName?: string;
  companyName?: string;
  className?: string;
  status: string;
  /** Photo élève (URL signée), si disponible. */
  photoUrl?: string | null;
  /** Pastille courte : Stage | Convention | Horaires | E-mail tuteur. */
  depositKind?: string | null;
  tutorEmailChangePending?: boolean;
  scheduleChangePending?: boolean;
};

export type StagesHubBoard = {
  viewer: string;
  viewerSecteurLabel?: string | null;
  permissions: StagesHubPermissions;
  counts: Record<string, number>;
  myPendingSignatures?: PendingStageSignature[];
  pendingOffers: Array<{ id: string; companyName: string; kind: string; targetLevels: string[] }>;
  adminQueue: StagesHubBoardCard[];
  signaturesPending: StagesHubBoardCard[];
  conventions: Array<{
    id: string;
    studentName: string;
    className: string;
    companyName: string;
    status: string;
    periodStart: string;
    periodEnd: string;
  }>;
};

export type StagesOfferForm = {
  kind: string;
  companyName: string;
  companyAddress: string;
  description: string;
  positionsCount: number;
  targetLevels: string[];
  periodStart: string;
  periodEnd: string;
  contactName: string;
  contactEmail: string;
  contactPhone: string;
  sector: string;
};
