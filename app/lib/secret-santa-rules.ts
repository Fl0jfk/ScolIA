import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";

/**
 * Qui gère les incompatibilités : le rôle admin dans l'établissement courant (ou la plateforme).
 * Ni la direction, ni le drapeau `orgAdmin` seul.
 */
export function canManageSantaRules(actor: { roles: string[]; platformAdmin: boolean }): boolean {
  return actor.platformAdmin || hasGlobalAdminRole(actor.roles) || hasMasterRole(actor.roles);
}
