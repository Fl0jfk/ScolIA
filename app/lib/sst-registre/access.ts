import { canAccessRhPersonalEspace, canAccessRhPilotageDashboard } from "@/app/lib/rh/rh-hub-access";

/** Tout collaborateur RH (profs, OGEC, direction) peut consulter / émarger / déposer une fiche. */
export function canAccessSstRegistre(roles: string[]): boolean {
  return canAccessRhPersonalEspace(roles);
}

/** Pilotage : suivi des signatures, visas, clôture des fiches + paramétrage proxy. */
export function canManageSstRegistre(roles: string[]): boolean {
  return canAccessRhPilotageDashboard(roles);
}

/** Droit de dépôt pour autrui : pilotage RH, ou liste paramétrée côté établissement. */
export function canDepositSstForOthers(params: {
  roles: string[];
  userId: string;
  proxyUserIds: ReadonlySet<string> | readonly string[];
}): boolean {
  if (canManageSstRegistre(params.roles)) return true;
  const uid = params.userId.trim();
  if (!uid) return false;
  if (params.proxyUserIds instanceof Set) return params.proxyUserIds.has(uid);
  return params.proxyUserIds.includes(uid);
}

export function primaryFonctionFromRoles(roles: string[]): string {
  const order = [
    "direction",
    "chef_etablissement",
    "directeur",
    "directrice",
    "admin",
    "administratif",
    "cpe",
    "professeur",
    "surveillant",
    "maintenance",
    "comptabilite",
    "accueil",
    "infirmerie",
    "psychologue",
  ];
  const normalized = roles.map((r) => r.trim().toLowerCase()).filter(Boolean);
  for (const key of order) {
    const hit = normalized.find((r) => r === key || r.includes(key));
    if (hit) {
      return hit
        .replace(/_/g, " ")
        .replace(/\b\w/g, (c) => c.toUpperCase());
    }
  }
  return normalized[0]
    ? normalized[0].replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())
    : "Personnel";
}
