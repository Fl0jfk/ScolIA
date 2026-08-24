import type { EleveConfig } from "@/app/lib/eleves-config";
import { buildEleveFolderName } from "@/app/lib/eleves-config";

export function normalizePersonPart(value: string): string {
  return String(value || "")
    .trim()
    .toUpperCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^A-Z0-9]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function identityKey(nom: string, prenom: string): string {
  return `${normalizePersonPart(nom)}§${normalizePersonPart(prenom)}`;
}

export function elevePhotoLookupKey(
  eleve: Pick<EleveConfig, "ine" | "nom" | "prenom" | "folderName">,
): string {
  const ine = eleve.ine?.trim().toUpperCase();
  if (ine) return `ine:${ine}`;
  return `name:${identityKey(eleve.nom, eleve.prenom)}`;
}

/** Extrait NOM + PRENOM depuis un nom de fichier photo. */
export function parsePhotoFilename(filename: string): { nom: string; prenom: string } | null {
  const base = filename
    .replace(/^.*[\\/]/, "")
    .replace(/\.[^.]+$/, "")
    .replace(/\+/g, " ")
    .trim();
  if (!base) return null;

  const parts = base
    .split(/[_\-\s.]+/)
    .map((p) => p.trim())
    .filter(Boolean);
  if (parts.length < 2) return null;

  return { nom: parts[0]!, prenom: parts.slice(1).join(" ") };
}

export function matchEleveForPhoto(
  eleves: EleveConfig[],
  nom: string,
  prenom: string,
): EleveConfig | null {
  const key = identityKey(nom, prenom);
  const hits = eleves.filter((e) => identityKey(e.nom, e.prenom) === key);
  if (hits.length >= 1) return hits[0]!;
  const keyRev = identityKey(prenom, nom);
  const hitsRev = eleves.filter((e) => identityKey(e.nom, e.prenom) === keyRev);
  return hitsRev[0] ?? null;
}

export function photoRelativePathForEleve(eleve: EleveConfig): string {
  const folder = eleve.folderName || buildEleveFolderName(eleve.nom, eleve.prenom);
  const safe = folder
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-zA-Z0-9]+/g, "_")
    .replace(/^_|_$/g, "");
  return `eleves/photos/${safe || "unknown"}.jpg`;
}
