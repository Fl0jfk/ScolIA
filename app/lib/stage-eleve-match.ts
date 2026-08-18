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
import { matchEleveFromDocument } from "@/app/lib/ocr-eleve-match";

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
  const decision = matchEleveFromDocument({
    text: `${convention.student.lastName} ${convention.student.firstName} ${convention.student.className || ""}`,
    eleves: allEleves,
    extracted: {
      nom: convention.student.lastName,
      prenom: convention.student.firstName,
      classe: convention.student.className,
      ine: ineFromConvention(convention),
      origine: "interne",
    },
  });
  const best = decision.decision === "auto" ? decision.eleve : null;
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

  const decision = matchEleveFromDocument({
    text: `${convention.student.lastName} ${convention.student.firstName} ${convention.student.className || ""}`,
    eleves: eleves.length > 0 ? eleves : allEleves,
    extracted: {
      nom: convention.student.lastName,
      prenom: convention.student.firstName,
      classe: convention.student.className,
      ine,
      origine: "interne",
    },
  });
  const best = decision.decision === "auto" ? decision.eleve : null;
  const folderPath = best && profile
    ? oneDrivePathForEleve(profile.basePath, resolveEleveFolderName(best))
    : null;

  return {
    matchedEleve: best,
    folderPath,
    debug: {
      matchMethod: decision.matchedBy || "nom_prenom",
      ine: ine || null,
      totalEleves: allEleves.length,
      poolSize: eleves.length,
      secteurFilterApplied,
      secteur: profile?.secteur ?? null,
      decision: decision.decision,
      candidates: decision.candidates,
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
