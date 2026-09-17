import path from "node:path";
import type { InscriptionLevelId } from "@/app/lib/document-templates/types";

export type InscriptionLevelCycle = "ecole" | "college" | "lycee";

export type InscriptionLevelMeta = {
  id: InscriptionLevelId;
  label: string;
  /** Cycle pour regrouper l’UI. */
  cycle: InscriptionLevelCycle;
  /** Nom de fichier sous assets/document-templates/inscription/ (absent = pas de PDF embarqué). */
  fileName?: string;
};

export const INSCRIPTION_LEVELS: InscriptionLevelMeta[] = [
  { id: "ps", label: "Petite section", cycle: "ecole" },
  { id: "ms", label: "Moyenne section", cycle: "ecole" },
  { id: "gs", label: "Grande section", cycle: "ecole" },
  { id: "cp", label: "CP", cycle: "ecole" },
  { id: "ce1", label: "CE1", cycle: "ecole" },
  { id: "ce2", label: "CE2", cycle: "ecole" },
  { id: "cm1", label: "CM1", cycle: "ecole" },
  { id: "cm2", label: "CM2", cycle: "ecole" },
  { id: "sixieme", label: "Sixième", cycle: "college", fileName: "sixieme.pdf" },
  { id: "cinquieme", label: "Cinquième", cycle: "college", fileName: "cinquieme.pdf" },
  { id: "quatrieme", label: "Quatrième", cycle: "college", fileName: "quatrieme.pdf" },
  { id: "troisieme", label: "Troisième", cycle: "college", fileName: "troisieme.pdf" },
  { id: "seconde", label: "Seconde", cycle: "lycee", fileName: "seconde.pdf" },
  {
    id: "premiere-generale",
    label: "Première Générale",
    cycle: "lycee",
    fileName: "premiere-generale.pdf",
  },
  {
    id: "premiere-st2s",
    label: "Première ST2S",
    cycle: "lycee",
    fileName: "premiere-st2s.pdf",
  },
  {
    id: "terminale-generale",
    label: "Terminale Générale",
    cycle: "lycee",
    fileName: "terminale-generale.pdf",
  },
  {
    id: "terminale-st2s",
    label: "Terminale ST2S",
    cycle: "lycee",
    fileName: "terminale-st2s.pdf",
  },
];

export function getInscriptionLevelMeta(
  id: string,
): InscriptionLevelMeta | undefined {
  return INSCRIPTION_LEVELS.find((l) => l.id === id);
}

export function isInscriptionLevelId(id: string): id is InscriptionLevelId {
  return INSCRIPTION_LEVELS.some((l) => l.id === id);
}

/** Cycle associé au slug de direction RDV (`ecole` | `college` | `lycee`). */
export function inscriptionCycleForDirectionSlug(
  slug: string,
): InscriptionLevelCycle | null {
  const s = slug.trim().toLowerCase();
  if (s === "ecole" || s === "college" || s === "lycee") return s;
  return null;
}

export function inscriptionLevelsForDirectionSlug(slug: string): InscriptionLevelMeta[] {
  const cycle = inscriptionCycleForDirectionSlug(slug);
  if (!cycle) return [...INSCRIPTION_LEVELS];
  return INSCRIPTION_LEVELS.filter((l) => l.cycle === cycle);
}

/** Chemin absolu du PDF source embarqué (repo). */
export function inscriptionSourcePath(levelId: InscriptionLevelId): string {
  const meta = getInscriptionLevelMeta(levelId);
  if (!meta?.fileName) throw new Error(`Niveau inconnu ou sans PDF: ${levelId}`);
  return path.join(
    process.cwd(),
    "assets",
    "document-templates",
    "inscription",
    meta.fileName,
  );
}
