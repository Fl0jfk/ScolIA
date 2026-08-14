import type { EleveConfig } from "@/app/lib/eleves-config";

export type ClassLevel = "ecole" | "college" | "lycee";

export type ClassAllocationCampaignConfig = {
  id: string;
  label: string;
  isOpen: boolean;
  openAt?: string;
  closeAt?: string;
  levels: {
    level: ClassLevel;
    sourceClassPrefixes: string[];
    targetClasses: string[];
  }[];
  teacherCatalog: string[];
};

export type ParentWish = {
  campaignId: string;
  studentIne: string;
  studentName: string;
  /** E-mail parent validé à la connexion. */
  parentEmail?: string;
  level: ClassLevel;
  preferredStudentInes: string[];
  avoidStudentInes: string[];
  /** Saisie libre parent (audit). */
  preferredStudentInputs?: string[];
  avoidStudentInputs?: string[];
  preferredTeacher?: string;
  avoidTeacher?: string;
  preferredTeacherInput?: string;
  avoidTeacherInput?: string;
  /** Noms non résolus par l'IA — conservés pour relecture admin. */
  unresolvedPeerInputs?: string[];
  unresolvedTeacherInputs?: string[];
  submittedAt: string;
};

export type StaffWish = {
  campaignId: string;
  studentIne: string;
  level: ClassLevel;
  actorUserId: string;
  actorRole: "professeur" | "direction" | "admin";
  separateFromInes: string[];
  willingToTake?: boolean | null;
  note?: string;
  submittedAt: string;
};

export type StudentScoreEntry = {
  studentIne: string;
  level: ClassLevel;
  score: number;
  gender?: "F" | "M" | "X";
  source: "manual";
  updatedAt: string;
};

export type ClassAllocationEntry = {
  className: string;
  studentInes: string[];
  stats: {
    count: number;
    avgScore: number;
    girls: number;
    boys: number;
    other: number;
  };
};

export type ClassAllocationRun = {
  id: string;
  campaignId: string;
  createdAt: string;
  levelResults: Record<ClassLevel, ClassAllocationEntry[]>;
  diagnostics: string[];
  score: number;
};

type ClassAllocationPublicStudent = Pick<
  EleveConfig,
  "ine" | "nom" | "prenom" | "classe"
>;

export const CLASS_LEVELS: ClassLevel[] = ["ecole", "college", "lycee"];
