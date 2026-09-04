/**
 * Périmètre dossier / administratif pour les professeurs (client + serveur).
 * Un prof voit ses classes — pas un 403 sur le reste : les élèves hors classe
 * sont simplement hors de sa liste / introuvables pour lui.
 */

import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

/**
 * Accès professeurs au module Dossiers élèves (classes assignées, fiche limitée).
 */
export const ELEVE_DOSSIER_ENABLED_FOR_PROFESSEURS = true;

/**
 * TEMPORAIRE — démarrage établissement : les profs voient tous les élèves
 * (pas seulement leurs classes roster / EDT / stages).
 * Sections toujours limitées à Synthèse + Scolarité.
 * Remettre à `false` dès que l’affectation classes ↔ profs est fiable.
 */
export const PROFESSEUR_DOSSIER_SEE_ALL_CLASSES_TEMPORARY = true;

/**
 * Onglets / sections autorisés pour un professeur « pur »
 * (pas direction / admin / administratif…) : Synthèse (= identité) + Scolarité.
 */
export const PROFESSEUR_DOSSIER_SECTIONS = ["identite", "scolarite"] as const;

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
    hasRole(opts.roles, "accueil") ||
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
 * - Dossiers élèves de ses classes (Synthèse + Scolarité)
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

/** Le viewer prof a-t-il le droit d’ouvrir Dossiers élèves ? */
export function professeurMayAccessEleveDossier(opts: {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): boolean {
  if (!isProfesseurScopedDossierViewer(opts)) return true;
  return ELEVE_DOSSIER_ENABLED_FOR_PROFESSEURS;
}

/** Modules du pilier Administratif visibles pour un prof (vue réduite). */
export const ADMINISTRATIF_PROF_MODULE_IDS = ELEVE_DOSSIER_ENABLED_FOR_PROFESSEURS
  ? (["eleve-dossier", "certificates"] as const)
  : (["certificates"] as const);
