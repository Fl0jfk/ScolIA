import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";

/** Rôles famille / portail — pas de 2FA TOTP obligatoire pour l’instant. */
const FAMILY_ONLY_ROLES = new Set(["parent", "eleve"]);

export const MFA_TRUST_STAFF_SECONDS = 60 * 60 * 24 * 30; // 30 jours
export const MFA_TRUST_DIRECTION_SECONDS = 60 * 60 * 24 * 7; // 7 jours

export type MfaTrustPolicy = {
  /** Si false, l’appareil ne peut jamais être « oublié » : MFA à chaque connexion. */
  allowTrust: boolean;
  maxAgeSeconds: number;
  /** Libellé court UI. */
  label: string;
  /** Texte d’aide sous la case. */
  hint: string;
};

/**
 * Indique si le compte doit activer la 2FA (TOTP).
 * Obligatoire pour tout le personnel intranet ; exclus uniquement les comptes
 * dont les rôles sont purement `parent` et/ou `eleve`.
 */
export function roleRequiresTwoFactor(opts: {
  platformAdmin: boolean;
  orgAdmin: boolean;
  roles: string[];
}): boolean {
  if (opts.platformAdmin || opts.orgAdmin) return true;

  const roles = opts.roles.filter(Boolean);
  if (roles.length === 0) return true;

  const hasStaffRole = roles.some((role) => !FAMILY_ONLY_ROLES.has(role));
  return hasStaffRole;
}

function isDirectionRoleSet(roles: string[]): boolean {
  return roles.some((r) =>
    INTRANET_DIRECTION_SLUGS.includes(r as (typeof INTRANET_DIRECTION_SLUGS)[number]),
  );
}

/**
 * Compte « admin établissement » (ou master plateforme) : MFA à chaque connexion.
 * Ne pas se fier au seul flag orgAdmin (trop large).
 */
export function isStrictAdminForMfa(opts: {
  platformAdmin?: boolean;
  roles: string[];
}): boolean {
  if (opts.platformAdmin) return true;
  return hasGlobalAdminRole(opts.roles) || hasMasterRole(opts.roles);
}

/**
 * Durée « se souvenir de cet appareil » selon le profil.
 * - admin / master / platform : jamais (MFA à chaque login)
 * - direction_* : 7 jours
 * - reste du personnel : 30 jours
 */
export function resolveMfaTrustPolicy(opts: {
  platformAdmin?: boolean;
  roles: string[];
}): MfaTrustPolicy {
  if (isStrictAdminForMfa(opts)) {
    return {
      allowTrust: false,
      maxAgeSeconds: 0,
      label: "MFA à chaque connexion",
      hint: "Compte administrateur : la double authentification est exigée à chaque connexion.",
    };
  }
  if (isDirectionRoleSet(opts.roles)) {
    return {
      allowTrust: true,
      maxAgeSeconds: MFA_TRUST_DIRECTION_SECONDS,
      label: "Se souvenir de cet appareil 7 jours",
      hint: "Compte direction : appareil de confiance limité à 7 jours.",
    };
  }
  return {
    allowTrust: true,
    maxAgeSeconds: MFA_TRUST_STAFF_SECONDS,
    label: "Se souvenir de cet appareil 30 jours",
    hint: "Sur cet appareil, la MFA ne sera pas redemandée pendant 30 jours.",
  };
}
