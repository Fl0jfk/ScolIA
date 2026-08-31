import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole, hasMasterRole, normRole } from "@/app/lib/intranet-role-utils";

/** Rôles famille / portail — pas de 2FA TOTP obligatoire pour l’instant. */
const FAMILY_ONLY_ROLES = new Set(["parent", "eleve"]);

/**
 * Rôles enseignants : MFA facultative (ceux qui l’ont déjà activée la gardent à la connexion).
 * Un professeur qui a aussi un rôle direction / admin / personnel reste soumis à la MFA obligatoire.
 */
const TEACHER_MFA_OPTIONAL_ROLES = new Set(["professeur", "enseignant"]);

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

export function isTeacherMfaOptionalRole(role: string): boolean {
  const n = normRole(role);
  return TEACHER_MFA_OPTIONAL_ROLES.has(n) || n.includes("professeur") || n.includes("enseignant");
}

/**
 * Indique si le compte doit activer la 2FA (TOTP).
 * Obligatoire pour admin, direction et personnel (hors professeur « pur »).
 * Facultative pour les professeurs / enseignants — y compris cumulés avec parent/élève.
 * Si la MFA est déjà activée, Better-Auth continue de la demander à la connexion.
 */
export function roleRequiresTwoFactor(opts: {
  platformAdmin: boolean;
  orgAdmin: boolean;
  roles: string[];
}): boolean {
  if (opts.platformAdmin || opts.orgAdmin) return true;

  const roles = opts.roles.filter(Boolean);
  if (roles.length === 0) return true;

  const staffRoles = roles.filter((role) => !FAMILY_ONLY_ROLES.has(role));
  if (staffRoles.length === 0) return false;

  const hasPrivilegedStaff = staffRoles.some((role) => !isTeacherMfaOptionalRole(role));
  return hasPrivilegedStaff;
}

/** Compte encore « en attente » d’activation (MDP / e-mail / MFA obligatoire). */
export function isAccountActivationPending(opts: {
  emailVerified: boolean;
  mustChangePassword: boolean;
  twoFactorEnabled: boolean;
  platformAdmin: boolean;
  orgAdmin: boolean;
  roles: string[];
}): boolean {
  if (!opts.emailVerified || opts.mustChangePassword) return true;
  return (
    roleRequiresTwoFactor({
      platformAdmin: opts.platformAdmin,
      orgAdmin: opts.orgAdmin,
      roles: opts.roles,
    }) && !opts.twoFactorEnabled
  );
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
