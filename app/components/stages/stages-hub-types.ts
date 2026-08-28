import type { PendingStageSignature } from "@/app/lib/stage-pending-signatures";

export type StageTab = "board" | "classe" | "offers" | "conventions" | "settings";

export type StagesHubPermissions = {
  canModerateOffers: boolean;
  canReviewPreconvention: boolean;
  canViewAllConventions: boolean;
  canViewReferentConventions: boolean;
  canDepositOffer: boolean;
  canFileToOneDrive: boolean;
  canManageStageSettings: boolean;
  canManageReferents: boolean;
  referentOnly: boolean;
  canViewClassRoster: boolean;
};

export type StagesHubBoard = {
  viewer: string;
  viewerSecteurLabel?: string | null;
  permissions: StagesHubPermissions;
  counts: Record<string, number>;
  myPendingSignatures?: PendingStageSignature[];
  pendingOffers: Array<{ id: string; companyName: string; kind: string; targetLevels: string[] }>;
  adminQueue: Array<{
    id: string;
    student?: { firstName: string; lastName: string };
    company?: { name: string };
    studentName?: string;
    companyName?: string;
    status: string;
  }>;
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
