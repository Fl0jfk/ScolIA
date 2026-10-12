import { canAccessRhPersonalEspace, canAccessRhPilotageDashboard } from "@/app/lib/rh/rh-hub-access";

/** Tout collaborateur RH (profs, OGEC, direction) peut consulter / émarger / déposer une fiche. */
export function canAccessSstRegistre(roles: string[]): boolean {
  return canAccessRhPersonalEspace(roles);
}

/** Pilotage : suivi des signatures, visas, clôture des fiches. */
export function canManageSstRegistre(roles: string[]): boolean {
  return canAccessRhPilotageDashboard(roles);
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
