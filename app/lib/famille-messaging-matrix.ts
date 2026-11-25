/**
 * Matrice messagerie familles — qui peut écrire / diffuser.
 * Module pur (pas de server-only) pour tests + UI client.
 */

import { hasGlobalAdminRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

export const FAMILLE_MESSAGING_ROLE_OPTIONS = [
  { id: "admin", label: "Admin" },
  { id: "direction", label: "Direction" },
  { id: "cpe", label: "CPE / vie scolaire" },
  { id: "administratif", label: "Administratif" },
  { id: "professeur", label: "Professeur" },
] as const;

export type FamilleMessagingSettingsDto = {
  rolesCanInitiate: string[];
  profOwnClassesOnly: boolean;
  allowBroadcast: boolean;
  allowParentAttachments: boolean;
};

export const DEFAULT_FAMILLE_MESSAGING_SETTINGS: FamilleMessagingSettingsDto = {
  rolesCanInitiate: ["admin", "cpe", "direction", "directeur", "directrice", "administratif"],
  profOwnClassesOnly: true,
  allowBroadcast: true,
  allowParentAttachments: true,
};

function normRole(r: string): string {
  return String(r || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");
}

function classKey(v: string): string {
  return v
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, "");
}

function classeInAssigned(studentClasse: string | undefined, assignedClasses: string[]): boolean {
  if (!assignedClasses.length) return false;
  const student = classKey(String(studentClasse ?? ""));
  if (!student) return false;
  return assignedClasses.some((c) => {
    const target = classKey(c);
    if (!target) return false;
    if (student === target) return true;
    return (
      student.length >= 2 &&
      target.length >= 2 &&
      (student.startsWith(target) || target.startsWith(student))
    );
  });
}

/** Un rôle utilisateur matche-t-il une entrée de matrice ? */
export function roleMatchesMatrixEntry(userRoles: string[], entry: string): boolean {
  const e = normRole(entry);
  if (!e) return false;
  if (e === "direction" || e === "directeur" || e === "directrice") {
    return userRoles.some(
      (r) =>
        INTRANET_DIRECTION_SLUGS.includes(r as (typeof INTRANET_DIRECTION_SLUGS)[number]) ||
        normRole(r).includes("direction") ||
        normRole(r).includes("directeur") ||
        normRole(r).includes("directrice"),
    );
  }
  if (e === "admin") {
    return hasGlobalAdminRole(userRoles) || userRoles.some((r) => normRole(r) === "admin");
  }
  if (e === "cpe" || e === "viescolaire") {
    return userRoles.some((r) => {
      const n = normRole(r);
      return n.includes("cpe") || n.includes("viescolaire");
    });
  }
  if (e === "administratif" || e === "secretariat") {
    return userRoles.some((r) => {
      const n = normRole(r);
      return n.includes("administratif") || n.includes("secretariat");
    });
  }
  if (e === "professeur") {
    return userRoles.some((r) => normRole(r).includes("professeur"));
  }
  return userRoles.some((r) => normRole(r).includes(e) || normRole(r) === e);
}

export function canInitiateFromMatrix(
  roles: string[],
  settings: FamilleMessagingSettingsDto,
  opts?: { orgAdmin?: boolean },
): boolean {
  if (opts?.orgAdmin || hasGlobalAdminRole(roles)) return true;
  return settings.rolesCanInitiate.some((entry) => roleMatchesMatrixEntry(roles, entry));
}

export function canBroadcastFromMatrix(
  roles: string[],
  settings: FamilleMessagingSettingsDto,
  opts?: { orgAdmin?: boolean },
): boolean {
  if (!settings.allowBroadcast) return false;
  if (opts?.orgAdmin || hasGlobalAdminRole(roles)) return true;
  const broadcasters = settings.rolesCanInitiate.filter(
    (r) => !normRole(r).includes("professeur"),
  );
  return broadcasters.some((entry) => roleMatchesMatrixEntry(roles, entry));
}

export function isProfesseurOnly(roles: string[]): boolean {
  if (hasGlobalAdminRole(roles)) return false;
  if (
    roles.some((r) =>
      INTRANET_DIRECTION_SLUGS.includes(r as (typeof INTRANET_DIRECTION_SLUGS)[number]),
    )
  ) {
    return false;
  }
  if (
    roles.some((r) => {
      const n = normRole(r);
      return (
        n.includes("cpe") ||
        n.includes("administratif") ||
        n.includes("secretariat") ||
        n.includes("viescolaire")
      );
    })
  ) {
    return false;
  }
  return roles.some((r) => normRole(r).includes("professeur"));
}

function foldSearch(v: string): string {
  return v
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

/** Recherche (libellé, élèves) + pagination côté serveur de la liste affichée. */
export function searchPaginateFoyers<
  T extends { label: string; eleves: Array<{ nom: string; prenom: string }> },
>(foyers: T[], opts: { q?: string; limit?: number; offset?: number }): { items: T[]; total: number } {
  const q = foldSearch((opts.q ?? "").trim());
  const filtered = q
    ? foyers.filter((f) =>
        foldSearch([f.label, ...f.eleves.map((e) => `${e.prenom} ${e.nom}`)].join(" ")).includes(q),
      )
    : foyers;
  const limit = Math.min(300, Math.max(1, Math.floor(opts.limit ?? 100)));
  const offset = Math.max(0, Math.floor(opts.offset ?? 0));
  return { items: filtered.slice(offset, offset + limit), total: filtered.length };
}

/** Filtre foyers pour un prof (classes assignées). */
export function filterFoyersForProfClasses<
  T extends { eleves: Array<{ classe: string | null }> },
>(foyers: T[], assignedClasses: string[]): T[] {
  if (!assignedClasses.length) return [];
  return foyers.filter((f) =>
    f.eleves.some((e) => classeInAssigned(e.classe ?? undefined, assignedClasses)),
  );
}
