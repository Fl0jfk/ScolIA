/** Chemins OneDrive RH (drive de l'attachée de gestion). */

export const RH_DEFAULT_BASE_PATH = "Dossier personnel";
const RH_TEMP_FOLDER = "Temp";
const RH_INDEX_FILE = "personnel-index.json";
const RH_META_FILE = "meta-rh.json";

export function normalizeRhBasePath(basePath?: string | null): string {
  const p = String(basePath ?? RH_DEFAULT_BASE_PATH).trim().replace(/^\/+|\/+$/g, "");
  return p || RH_DEFAULT_BASE_PATH;
}

export function rhIndexPath(basePath?: string | null): string {
  return `${normalizeRhBasePath(basePath)}/${RH_INDEX_FILE}`;
}

export function rhTempPath(basePath?: string | null, fileName?: string): string {
  const base = `${normalizeRhBasePath(basePath)}/${RH_TEMP_FOLDER}`;
  if (!fileName) return base;
  return `${base}/${fileName}`;
}

export function rhPersonnelFolderPath(folderName: string, basePath?: string | null): string {
  return `${normalizeRhBasePath(basePath)}/${folderName.trim()}`;
}

export function rhMetaPath(folderName: string, basePath?: string | null): string {
  return `${rhPersonnelFolderPath(folderName, basePath)}/${RH_META_FILE}`;
}

export function rhDocumentsRoot(folderName: string, basePath?: string | null): string {
  return `${rhPersonnelFolderPath(folderName, basePath)}/documents`;
}

export type RhDocSubfolder =
  | "contrats"
  | "formations"
  | "habilitations"
  | "medecine"
  | "personnels"
  | "divers";

export function rhDocumentPath(
  folderName: string,
  subfolder: RhDocSubfolder,
  fileName: string,
  basePath?: string | null,
): string {
  return `${rhDocumentsRoot(folderName, basePath)}/${subfolder}/${fileName}`;
}
