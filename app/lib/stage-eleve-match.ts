import type { EleveConfig } from "@/app/lib/eleves-config";
import { resolveEleveFolderName } from "@/app/lib/eleves-config";
import { loadElevesRegistry, findEleveByIne } from "@/app/lib/eleves-registry";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import {
  buildElevesPoolForOcrMatching,
  inferSecteurFromFolderName,
  oneDrivePathForEleve,
  resolveEleveSecteur,
  type Secteur,
} from "@/app/lib/onedrive-eleves";
import {
  getOneDriveProfileForSecteur,
  type OneDriveUserProfile,
} from "@/app/lib/onedrive-user-profiles";
import type { StageConvention } from "@/app/lib/stage-types";

function normalize(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

function levenshtein(a: string, b: string): number {
  if (a === b) return 0;
  if (!a.length) return b.length;
  if (!b.length) return a.length;
  let prev = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    const curr = [i];
    for (let j = 1; j <= b.length; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      curr[j] = Math.min(curr[j - 1] + 1, prev[j] + 1, prev[j - 1] + cost);
    }
    prev = curr;
  }
  return prev[b.length];
}

/**
 * Proximité stricte entre deux chaînes normalisées.
 * Tolère 1 erreur pour ≤6 chars, 2 pour ≤12 chars, 3 au-delà.
 */
function closeness(a: string, b: string): number {
  if (!a || !b) return 0;
  if (a === b) return 1;
  const maxLen = Math.max(a.length, b.length);
  if (maxLen === 0) return 0;
  const dist = levenshtein(a, b);
  const maxAllowed = maxLen <= 6 ? 1 : maxLen <= 12 ? 2 : 3;
  if (dist > maxAllowed) return 0;
  return 1 - dist / maxLen;
}

/**
 * Score de similarité nom/prénom strict.
 * Exige NOM et PRÉNOM présents et matchant tous les deux (seuil closeness >= 0.88).
 * Score max = 4.0. Seuil de match recommandé : >= 3.2.
 */
function nameSimilarity(aNom: string, aPrenom: string, bNom: string, bPrenom: string): number {
  const an = normalize(aNom);
  const ap = normalize(aPrenom);
  const bn = normalize(bNom);
  const bp = normalize(bPrenom);

  if (!an || !ap || !bn || !bp) return 0;

  const cNom = closeness(an, bn);
  const cPrenom = closeness(ap, bp);

  if (cNom < 0.88 || cPrenom < 0.88) {
    // Tenter inversion nom/prénom
    const cNomInv = closeness(an, bp);
    const cPrenomInv = closeness(ap, bn);
    if (cNomInv >= 0.88 && cPrenomInv >= 0.88) {
      return cNomInv + cPrenomInv; // ≤ 2.0 : score insuffisant pour auto-match
    }
    return 0;
  }

  return 2 * cNom + 2 * cPrenom;
}

async function loadEleves(): Promise<EleveConfig[]> {
  return loadElevesRegistry();
}

export { findEleveByIne } from "@/app/lib/eleves-registry";

function ineFromConvention(convention: StageConvention): string {
  const fromMeta = convention.ocrMeta?.matchedEleveIne?.trim();
  if (fromMeta) return fromMeta;
  const raw = convention.ocrMeta?.raw;
  if (!raw || typeof raw !== "object") return "";
  const o = raw as Record<string, unknown>;
  for (const key of ["ine_eleve", "ine", "studentIne"]) {
    const v = String(o[key] ?? "").trim();
    if (v && v !== "non_trouvé" && v !== "non_trouve") return v;
  }
  return "";
}

/** Secteur cible (École / Collège / Lycée) pour le rangement OneDrive. */
export async function resolveConventionSecteur(
  convention: StageConvention,
): Promise<Secteur | null> {
  const ine = ineFromConvention(convention);
  if (ine) {
    const eleve = await findEleveByIne(ine);
    if (eleve) {
      const mefMap = await loadMefSecteurMap();
      const s = resolveEleveSecteur(eleve, mefMap);
      if (s) return s;
    }
  }

  const fromClass =
    inferSecteurFromFolderName(convention.student.className) ||
    inferSecteurFromFolderName(convention.student.level);
  if (fromClass) return fromClass;

  const allEleves = await loadEleves();
  const mefMap = await loadMefSecteurMap();
  const scored = allEleves
    .map((e) => ({
      eleve: e,
      score: nameSimilarity(
        convention.student.lastName,
        convention.student.firstName,
        e.nom,
        e.prenom,
      ),
    }))
    .filter((s) => s.score >= 3.2)
    .sort((a, b) => b.score - a.score);

  const best = scored[0]?.eleve;
  if (best) {
    const s = resolveEleveSecteur(best, mefMap);
    if (s) return s;
  }

  return null;
}

/** Profil OneDrive (arborescence) déduit de la convention, sans compte Clerk. */
export async function resolveOneDriveProfileForConvention(
  convention: StageConvention,
  clerkProfile?: OneDriveUserProfile | null,
): Promise<OneDriveUserProfile | null> {
  const secteur = await resolveConventionSecteur(convention);
  if (secteur) {
    const fromSecteur = getOneDriveProfileForSecteur(secteur);
    if (fromSecteur) return fromSecteur;
  }
  return clerkProfile ?? null;
}

export async function matchEleveForConvention(
  convention: StageConvention,
  odProfile: OneDriveUserProfile | null,
): Promise<{
  matchedEleve: EleveConfig | null;
  folderPath: string | null;
  debug: Record<string, unknown>;
}> {
  const allEleves = await loadEleves();
  const mefMap = await loadMefSecteurMap();
  const profile =
    odProfile ?? (await resolveOneDriveProfileForConvention(convention, null));
  const { eleves, secteurFilterApplied } = buildElevesPoolForOcrMatching(
    allEleves,
    profile,
    mefMap,
  );

  const ine = ineFromConvention(convention);
  if (ine) {
    const found =
      eleves.find((e) => e.ine?.trim().toUpperCase() === ine.toUpperCase()) ??
      allEleves.find((e) => e.ine?.trim().toUpperCase() === ine.toUpperCase()) ??
      null;
    if (found) {
      const folderPath = profile
        ? oneDrivePathForEleve(profile.basePath, resolveEleveFolderName(found))
        : null;
      return {
        matchedEleve: found,
        folderPath,
        debug: {
          matchMethod: "ine",
          ine,
          totalEleves: allEleves.length,
          poolSize: eleves.length,
          secteurFilterApplied,
          secteur: profile?.secteur ?? null,
        },
      };
    }
  }

  const scored = eleves
    .map((e) => ({
      eleve: e,
      score: nameSimilarity(
        convention.student.lastName,
        convention.student.firstName,
        e.nom,
        e.prenom,
      ),
    }))
    .filter((s) => s.score >= 3.2)
    .sort((a, b) => b.score - a.score);

  const best = scored[0]?.eleve ?? null;
  const folderPath = best && profile
    ? oneDrivePathForEleve(profile.basePath, resolveEleveFolderName(best))
    : null;

  return {
    matchedEleve: best,
    folderPath,
    debug: {
      matchMethod: "nom_prenom",
      ine: ine || null,
      totalEleves: allEleves.length,
      poolSize: eleves.length,
      secteurFilterApplied,
      secteur: profile?.secteur ?? null,
      candidates: scored.slice(0, 3).map((s) => ({
        nom: s.eleve.nom,
        prenom: s.eleve.prenom,
        ine: s.eleve.ine,
        folderName: s.eleve.folderName,
        score: s.score,
      })),
    },
  };
}

export function conventionOneDriveFileName(convention: StageConvention): string {
  const last = convention.student.lastName
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[<>:"/\\|?*]/g, "_")
    .trim();
  const first = convention.student.firstName
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[<>:"/\\|?*]/g, "_")
    .trim();
  const kind =
    convention.internshipKind === "job_ete"
      ? "Job ete"
      : convention.internshipKind === "pfmp"
        ? "Convention PFMP"
        : "Convention stage";
  const period = `${convention.schedule.periodStart}_${convention.schedule.periodEnd}`.replace(/-/g, "");
  return `${kind} ${period} ${last} ${first}.pdf`;
}
