import path from "node:path";
import type { InscriptionLevelId } from "@/app/lib/document-templates/types";

export type InscriptionLevelMeta = {
  id: InscriptionLevelId;
  label: string;
  /** Cycle pour regrouper l’UI. */
  cycle: "college" | "lycee";
  /** Nom de fichier sous assets/document-templates/inscription/ */
  fileName: string;
};

export const INSCRIPTION_LEVELS: InscriptionLevelMeta[] = [
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

/** Chemin absolu du PDF source embarqué (repo). */
export function inscriptionSourcePath(levelId: InscriptionLevelId): string {
  const meta = getInscriptionLevelMeta(levelId);
  if (!meta) throw new Error(`Niveau inconnu: ${levelId}`);
  return path.join(
    process.cwd(),
    "assets",
    "document-templates",
    "inscription",
    meta.fileName,
  );
}
