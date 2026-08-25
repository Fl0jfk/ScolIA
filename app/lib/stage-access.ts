import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

const DIRECTIONS = [...INTRANET_DIRECTION_SLUGS];

type StageViewerRole =
  | "parent"
  | "administratif"
  | "direction"
  | "professeur"
  | "staff"
  | "externe";

export function resolveStageViewerRole(roles: string[]): StageViewerRole | null {
  if (roles.includes("parent")) return "parent";
  if (roles.some((r) => DIRECTIONS.includes(r as (typeof DIRECTIONS)[number]))) return "direction";
  if (roles.includes("administratif")) return "administratif";
  if (roles.includes("professeur")) return "professeur";
  if (
    roles.includes("surveillant") ||
    roles.includes("cpe") ||
    roles.includes("comptabilite") ||
    roles.includes("maintenance")
  ) {
    return "staff";
  }
  return null;
}

export function canModerateOffers(roles: string[]) {
  return roles.some((r) => DIRECTIONS.includes(r as (typeof DIRECTIONS)[number]));
}

export function canReviewPreconvention(roles: string[]) {
  return roles.includes("administratif") || canModerateOffers(roles);
}

export function canViewAllConventions(roles: string[]) {
  return canReviewPreconvention(roles) || roles.includes("surveillant") || roles.includes("cpe");
}

export function canViewReferentConventions(roles: string[]) {
  return roles.includes("professeur");
}

export function canCreateOffer(roles: string[]) {
  return roles.includes("parent");
}

export function canCreateConventionAsStaff(roles: string[]) {
  return (
    canReviewPreconvention(roles) ||
    roles.includes("professeur") ||
    roles.includes("surveillant") ||
    roles.includes("cpe")
  );
}

/** Envoi convention signée vers dossier élève OneDrive (flux OCR). */
export function canFileConventionToOneDrive(roles: string[]) {
  return roles.includes("administratif") || canModerateOffers(roles);
}
