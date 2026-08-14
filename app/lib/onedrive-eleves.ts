import type { EleveConfig } from "@/app/lib/eleves-config";
import { normMefCode } from "@/app/lib/mef-secteurs";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type EleveSecteurInput = EleveConfig & { secteur?: string; mef?: string };

function norm(s: string) {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

/** Détecte Lycée / Collège / École depuis le nom de dossier (ex. « Dupont — 6e A » → collège). */
export function inferSecteurFromFolderName(folderName: string): Secteur | null {
  const f = norm(folderName);
  if (
    /\b(6e|5e|4e|3e|sixieme|cinquieme|quatrieme|troisieme|3 eme|4 eme|5 eme|6 eme)\b/.test(f) ||
    f.includes("college") ||
    f.includes("colleg")
  ) {
    return "college";
  }
  if (
    /\b(2nde|2de|seconde|1re|1ere|premiere|terminale|tle|tale|2 nde|1 re)\b/.test(f) ||
    f.includes("lycee") ||
    f.includes("lyc")
  ) {
    return "lycee";
  }
  if (
    /\b(cp|ce1|ce2|cm1|cm2|maternelle|gs|ms|ps|tps|primaire)\b/.test(f) ||
    f.includes("ecole") ||
    f.includes("elementaire") ||
    f.includes("cours preparatoire") ||
    f.includes("cours elementaire") ||
    f.includes("cours moyen") ||
    f.includes("petite section") ||
    f.includes("moyenne section") ||
    f.includes("grande section") ||
    f.includes("toute petite section")
  ) {
    return "ecole";
  }
  return null;
}

export function resolveEleveSecteur(
  eleve: EleveSecteurInput,
  mefMap?: Map<string, Secteur> | null,
): Secteur | null {
  const explicit = String(eleve.secteur ?? "").trim().toLowerCase();
  if (explicit === "lycee" || explicit === "college" || explicit === "ecole") return explicit;

  const mef = normMefCode(String(eleve.mef ?? eleve.formation ?? ""));
  if (mef && mefMap && mefMap.size > 0) {
    const fromMef = mefMap.get(mef);
    if (fromMef) return fromMef;
  }

  return inferSecteurFromFolderName(eleve.folderName);
}

/**
 * Pool de matching OCR.
 * Par défaut : toute la liste eleves.json (MEF non bloquant).
 * Filtre par secteur seulement si les MEF sont largement renseignés sur la liste
 * (optimisation future, sans bloquer tant que ce n'est pas le cas).
 */
export function buildElevesPoolForOcrMatching(
  allEleves: EleveSecteurInput[],
  odProfile: { secteur: Secteur } | null,
  mefMap?: Map<string, Secteur> | null,
): { eleves: EleveSecteurInput[]; secteurFilterApplied: boolean } {
  if (!odProfile || !mefMap || mefMap.size === 0 || allEleves.length === 0) {
    return { eleves: allEleves, secteurFilterApplied: false };
  }

  const withResolvableMef = allEleves.filter((e) => {
    const mef = normMefCode(String(e.mef ?? e.formation ?? ""));
    return Boolean(mef && mefMap.has(mef));
  });

  const mefCoverageEnough =
    withResolvableMef.length >= Math.max(10, Math.ceil(allEleves.length * 0.2));

  if (!mefCoverageEnough) {
    return { eleves: allEleves, secteurFilterApplied: false };
  }

  const scoped = filterElevesForSecteur(allEleves, odProfile.secteur, mefMap);
  if (scoped.length === 0) {
    return { eleves: allEleves, secteurFilterApplied: false };
  }

  return { eleves: scoped, secteurFilterApplied: true };
}

export function filterElevesForSecteur(
  eleves: EleveSecteurInput[],
  secteur: Secteur,
  mefMap?: Map<string, Secteur> | null,
): EleveSecteurInput[] {
  return eleves.filter((e) => resolveEleveSecteur(e, mefMap) === secteur);
}

export function oneDrivePathForEleve(basePath: string, folderName: string): string {
  const base = basePath.replace(/\/+$/, "");
  const folder = folderName.replace(/^\/+/, "").trim();
  return `${base}/${folder}`;
}
