import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type PilotagePieceKind =
  | "bulletin"
  | "pap"
  | "pai"
  | "pps"
  | "tap"
  | "certificat"
  | "convention"
  | "autre";

export type PilotagePiece = {
  id: string;
  name: string;
  eTag?: string;
  kind: PilotagePieceKind;
  lastModifiedDateTime?: string;
  size?: number;
  /** Chemin OneDrive secrétariat (indexation). */
  path?: string;
  /** Lien de consultation (partage organisation, créé à l’indexation). */
  shareUrl?: string;
  webUrl?: string;
};

export type PilotageMatiereMoyenne = {
  matiere: string;
  moyenne: number | null;
};

export type PilotageBulletinExtrait = {
  pieceId: string;
  sourceName: string;
  anneeScolaire?: string;
  periode?: string;
  classe?: string;
  moyenneGenerale?: number | null;
  matieres?: PilotageMatiereMoyenne[];
  absencesMention?: string;
  appreciation?: string;
};

export type PilotageDropSignal = {
  kind: "none" | "start" | "expected_cycle" | "drop";
  detail: string;
  from?: number;
  to?: number;
};

export type PilotageEleveDossier = {
  key: string;
  secteur: Secteur;
  ine?: string;
  nom: string;
  prenom: string;
  folderName: string;
  classe?: string;
  identityUpdatedAt: string;
  pieces: PilotagePiece[];
  bulletins: PilotageBulletinExtrait[];
  flags: {
    hasPap: boolean;
    hasPai: boolean;
    hasPps: boolean;
    hasTap: boolean;
    emptyDossier: boolean;
  };
  drop: PilotageDropSignal;
  synthese?: {
    text: string;
    updatedAt: string;
    sources: string[];
  };
  lastIndexedAt: string;
};

export type PilotageEleveSummary = {
  key: string;
  nom: string;
  prenom: string;
  classe: string;
  folderName: string;
  emptyDossier: boolean;
  hasBulletin: boolean;
  hasPapPaiPps: boolean;
  dropSignal: boolean;
  lastMoyenne?: number | null;
  lastPeriode?: string;
};

export type PilotageSecteurIndex = {
  secteur: Secteur;
  updatedAt: string;
  eleves: Record<string, PilotageEleveSummary>;
};

export type PilotageClassOverview = {
  secteur: Secteur;
  classe: string;
  eleves: PilotageEleveSummary[];
};

export type PilotageOverview = {
  secteurs: Secteur[];
  classes: Array<{
    secteur: Secteur;
    classe: string;
    count: number;
    alerts: number;
    missingBulletin: number;
    drops: number;
  }>;
  canWriteNotes: boolean;
  canIndex: boolean;
};
