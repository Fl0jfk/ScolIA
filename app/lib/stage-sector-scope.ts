import "server-only";

import { resolvePilotageSecteursForRoles } from "@/app/lib/pilotage-eleves-access";
import { inferSecteurFromFolderName } from "@/app/lib/onedrive-eleves";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import type { StageConvention, StageOffer } from "@/app/lib/stage-types";

const SECTEUR_LABELS: Record<Secteur, string> = {
  ecole: "École",
  college: "Collège",
  lycee: "Lycée",
};

/** Corrige la clé interne `lycee` pour l'affichage. */
export function stageSecteurLabel(secteur: Secteur): string {
  if (secteur === "lycee") return SECTEUR_LABELS.lycee;
  return SECTEUR_LABELS[secteur];
}

export function inferStageSecteurFromClass(className: string, level?: string): Secteur | null {
  return (
    inferSecteurFromFolderName(className.trim()) ||
    inferSecteurFromFolderName(String(level ?? "").trim()) ||
    null
  );
}

export function classNameMatchesStageSecteurs(className: string, secteurs: Secteur[]): boolean {
  if (secteurs.length === 0) return true;
  const secteur = inferSecteurFromFolderName(className.trim());
  if (!secteur) return false;
  return secteurs.includes(secteur);
}

export function conventionMatchesStageSecteurs(
  convention: StageConvention,
  secteurs: Secteur[],
): boolean {
  if (secteurs.length === 0) return true;
  const secteur = inferStageSecteurFromClass(
    convention.student.className,
    convention.student.level,
  );
  if (!secteur) return false;
  return secteurs.includes(secteur);
}

export function offerMatchesStageSecteurs(offer: StageOffer, secteurs: Secteur[]): boolean {
  if (secteurs.length === 0) return true;
  for (const level of offer.targetLevels) {
    const secteur = inferSecteurFromFolderName(String(level ?? "").trim());
    if (secteur && secteurs.includes(secteur)) return true;
  }
  return false;
}

export async function resolveStageViewerSecteurs(
  roles: string[],
  userId: string,
): Promise<Secteur[]> {
  return resolvePilotageSecteursForRoles(roles, userId);
}

export function stageViewerSecteurSummary(secteurs: Secteur[]): string | null {
  if (secteurs.length === 0) return null;
  return secteurs.map((s) => stageSecteurLabel(s)).join(" · ");
}
