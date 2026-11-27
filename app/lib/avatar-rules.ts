/** Droits sur les photos de profil — règles pures. */

import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";

export type AvatarActor = {
  userId: string;
  /** Rôles dans l'établissement courant. */
  roles: string[];
  platformAdmin: boolean;
};

export type AvatarTarget = {
  /** L'établissement courant est l'établissement principal du compte. */
  isHome: boolean;
  platformAdmin: boolean;
};

/**
 * Seul un administrateur de l'établissement courant gère les photos des autres. Le drapeau
 * `orgAdmin` du compte est ignoré : il vaut pour tous les établissements. La direction ne suffit pas.
 */
export function canManageOthersAvatars(actor: Omit<AvatarActor, "userId">): boolean {
  return actor.platformAdmin || hasGlobalAdminRole(actor.roles) || hasMasterRole(actor.roles);
}

/**
 * La photo est la même dans tous les établissements du compte : seul celui d'origine la gère, et
 * un compte de la plateforme n'est modifiable que par la plateforme.
 */
export function canManageAvatarOf(actor: AvatarActor, target: AvatarTarget): boolean {
  if (!canManageOthersAvatars(actor)) return false;
  return actor.platformAdmin || (target.isHome && !target.platformAdmin);
}

/** Adresse enregistrée dans `user.image` ; `version` force le rafraîchissement après un changement. */
export function avatarUrl(userId: string, updatedAt: Date): string {
  return `/api/avatars/${encodeURIComponent(userId)}?v=${updatedAt.getTime()}`;
}
