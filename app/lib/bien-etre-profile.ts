/** Détecte le profil élève bien-être (bot d'écoute, pas Nico institutionnel). */

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

const STAFF_ROLE_SLUGS = new Set([
  "admin",
  "administratif",
  "professeur",
  "direction_ecole",
  "direction_college",
  "direction_lycee",
  "comptabilite",
  "maintenance",
  "infirmerie",
  "education",
  "cpe",
  "parent",
  "master",
]);

/** Alias historique — délégué à la normalisation intranet. */
export function intranetRolesFromUnknown(meta: unknown): string[] {
  return intranetRolesFromMetadata(meta);
}

/** Élève sans rôle staff → agent bien-être dans la bulle IA. */
export function isEleveBienEtreProfile(roles: string[]): boolean {
  if (!roles.includes("eleve")) return false;
  return !roles.some((r) => STAFF_ROLE_SLUGS.has(r));
}
