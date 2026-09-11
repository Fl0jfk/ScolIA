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

/** Découpe un nom de fichier photo en jetons (NOM / prénom composés inclus). */
export function tokenizePhotoFilename(filename: string): string[] | null {
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
  return parts.length >= 2 ? parts : null;
}

/**
 * Extrait NOM + PRENOM depuis un nom de fichier photo.
 * Heuristique simple (1er jeton = nom) — le matching tente ensuite toutes les découpes
 * pour les noms composés (« LE ROUX Marie »).
 */
export function parsePhotoFilename(filename: string): { nom: string; prenom: string } | null {
  const parts = tokenizePhotoFilename(filename);
  if (!parts) return null;
  return { nom: parts[0]!, prenom: parts.slice(1).join(" ") };
}

function findByIdentityKey(eleves: EleveConfig[], key: string): EleveConfig[] {
  return eleves.filter((e) => identityKey(e.nom, e.prenom) === key);
}

/**
 * Associe une photo à un élève.
 * Pour les noms de famille à espaces (LE ROUX, DE LA …), on teste toutes les
 * façons de couper les jetons du fichier : sinon « LE ROUX Marie.jpg » devenait
 * nom=LE / prénom=ROUX Marie et ne matchait jamais.
 */
export function matchEleveForPhoto(
  eleves: EleveConfig[],
  nom: string,
  prenom: string,
): EleveConfig | null {
  const exactHits = findByIdentityKey(eleves, identityKey(nom, prenom));
  if (exactHits.length >= 1) return exactHits[0]!;

  const tokens = [
    ...normalizePersonPart(nom).split(" "),
    ...normalizePersonPart(prenom).split(" "),
  ].filter(Boolean);

  if (tokens.length >= 2) {
    // Préférer le nom le plus long qui matche de façon unique (LE ROUX > LE).
    let best: EleveConfig | null = null;
    let bestNomTokens = 0;
    for (let i = 1; i < tokens.length; i++) {
      const n = tokens.slice(0, i).join(" ");
      const p = tokens.slice(i).join(" ");
      const hits = findByIdentityKey(eleves, identityKey(n, p));
      if (hits.length === 1 && i >= bestNomTokens) {
        best = hits[0]!;
        bestNomTokens = i;
      }
    }
    if (best) return best;

    // Dernier recours : chaîne complète = « NOM PRENOM » dossier.
    const joined = normalizePersonPart(tokens.join(" "));
    const byFolder = eleves.filter((e) => {
      const folder = normalizePersonPart(e.folderName || `${e.nom} ${e.prenom}`);
      return folder === joined;
    });
    if (byFolder.length >= 1) return byFolder[0]!;
  }

  const hitsRev = findByIdentityKey(eleves, identityKey(prenom, nom));
  return hitsRev[0] ?? null;
}

/** Raccourci : fichier → élève (gère noms composés). */
export function matchEleveFromPhotoFilename(
  eleves: EleveConfig[],
  filename: string,
): EleveConfig | null {
  const parsed = parsePhotoFilename(filename);
  if (!parsed) return null;
  return matchEleveForPhoto(eleves, parsed.nom, parsed.prenom);
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
