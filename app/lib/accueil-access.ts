/**
 * Rôle « Accueil » (standard) — périmètre réduit vs administratif :
 * absences accueil, visu sorties / dossiers (liste seule), photocopies (réception), RH perso.
 */

import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

export const ACCUEIL_ROLE_SLUG = "accueil";

/** Rôles qui ouvrent le détail d’un séjour (pas seulement la liste). */
const TRAVELS_DETAIL_ROLES = new Set([
  "admin",
  "administratif",
  "comptabilite",
  "professeur",
  "cpe",
  "surveillant",
  "infirmerie",
  ...INTRANET_DIRECTION_SLUGS,
]);

/** Rôles qui ouvrent une fiche élève (pas seulement le tableau de bord / liste). */
const ELEVE_DOSSIER_DETAIL_ROLES = new Set([
  "admin",
  "administratif",
  "comptabilite",
  "professeur",
  "cpe",
  "surveillant",
  "infirmerie",
  "psychologue",
  ...INTRANET_DIRECTION_SLUGS,
]);

export function hasAccueilRole(roles: string[]): boolean {
  return hasRole(roles, ACCUEIL_ROLE_SLUG);
}

/**
 * Accueil seul : liste des sorties / dashboard global, sans entrer dans un dossier.
 * Si un rôle « plein » est aussi présent, le détail reste autorisé.
 */
export function canEnterTravelsDetail(opts: {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): boolean {
  if (opts.platformAdmin || opts.orgAdmin || hasGlobalAdminRole(opts.roles)) return true;
  if (opts.roles.some((r) => TRAVELS_DETAIL_ROLES.has(r))) return true;
  return false;
}

/**
 * Accueil seul : tableau de bord / liste dossiers, sans ouvrir une fiche élève.
 */
export function canOpenEleveDossierDetail(opts: {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): boolean {
  if (opts.platformAdmin || opts.orgAdmin || hasGlobalAdminRole(opts.roles)) return true;
  if (opts.roles.some((r) => ELEVE_DOSSIER_DETAIL_ROLES.has(r))) return true;
  return false;
}

/** Chemins détail séjour (`/travels/<id>` ou variantes simple/complex). */
export function isTravelsDetailPath(pathname: string): boolean {
  const p = pathname.replace(/\/+$/, "") || "/";
  if (p === "/travels" || p.startsWith("/travels/api") || p.startsWith("/api/travels")) {
    return false;
  }
  if (/^\/travels\/(simple|complex)(\/|$)/i.test(p)) return true;
  if (/^\/travels\/[^/]+$/i.test(p) && !/^\/travels\/(settings)?$/i.test(p)) return true;
  return false;
}

/** Fiche élève (`/eleves/dossier/<id>`), pas la liste `/eleves/dossiers`. */
export function isEleveDossierDetailPath(pathname: string): boolean {
  const p = pathname.replace(/\/+$/, "") || "/";
  return p === "/eleves/dossier" || p.startsWith("/eleves/dossier/");
}

/** API détail séjour (lecture d’un dossier). */
export function isTravelsDetailApiPath(pathname: string): boolean {
  const p = pathname.replace(/\/+$/, "") || "/";
  return (
    p === "/api/travels/get" ||
    p.startsWith("/api/travels/get/") ||
    p === "/api/travels/update" ||
    p.startsWith("/api/travels/update/") ||
    p === "/api/travels/delete" ||
    p.startsWith("/api/travels/delete/")
  );
}

/** API fiche élève. */
export function isEleveDossierDetailApiPath(pathname: string): boolean {
  const p = pathname.replace(/\/+$/, "") || "/";
  return /^\/api\/eleves\/[^/]+\/dossier(\/|$)/i.test(p);
}
