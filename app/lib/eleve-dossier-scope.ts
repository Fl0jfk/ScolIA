/**
 * Périmètre dossier / administratif pour les professeurs (client + serveur).
 * Un prof voit ses classes — pas un 403 sur le reste : les élèves hors classe
 * sont simplement hors de sa liste / introuvables pour lui.
 */

import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

/** Accès hub dossiers complet (accès docs, tous les élèves, filtres établissement). */
export function canViewFullElevesDossierHub(opts: {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): boolean {
  if (opts.platformAdmin || opts.orgAdmin || hasGlobalAdminRole(opts.roles)) return true;
  if (INTRANET_DIRECTION_SLUGS.some((slug) => opts.roles.includes(slug))) return true;
  return (
    hasRole(opts.roles, "administratif") ||
    hasRole(opts.roles, "surveillant") ||
    hasRole(opts.roles, "cpe") ||
    opts.roles.includes("admin")
  );
}

/**
 * Préinscriptions : uniquement administratif + direction (pas CPE / surveillant / prof…).
 */
export function canManageElevePreinscriptions(opts: {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): boolean {
  if (opts.platformAdmin || opts.orgAdmin || hasGlobalAdminRole(opts.roles)) return true;
  if (opts.roles.includes("admin")) return true;
  if (INTRANET_DIRECTION_SLUGS.some((slug) => opts.roles.includes(slug))) return true;
  return hasRole(opts.roles, "administratif");
}

/**
 * Professeur sans rôle staff élargi :
 * - Administratif réduit (dossiers de ses classes + notes)
 * - Pas de préinscriptions / réglages admin
 */
export function isProfesseurScopedDossierViewer(opts: {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): boolean {
  if (!hasRole(opts.roles, "professeur")) return false;
  return !canViewFullElevesDossierHub(opts);
}

/** Modules du pilier Administratif visibles pour un prof (vue réduite). */
export const ADMINISTRATIF_PROF_MODULE_IDS = ["eleve-dossier", "certificates"] as const;
