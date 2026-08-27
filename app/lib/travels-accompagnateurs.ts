/** Accompagnateurs de sortie : annuaire + saisie libre (« Autre »). */

import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

export type TravelsAccompagnateurSource = "directory" | "autre";

export type TravelsAccompagnateur = {
  /** Présent si choisi dans l’annuaire. */
  userId?: string;
  name: string;
  email?: string;
  source: TravelsAccompagnateurSource;
};

export type TravelsEscortDirectoryUser = {
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  roles: string[];
};

/** Filtres UI demandés pour les accompagnateurs. */
export type TravelsEscortRoleFilter = "all" | "administratif" | "direction" | "professeur" | "autre";

export const TRAVELS_ESCORT_ROLE_FILTERS: { id: TravelsEscortRoleFilter; label: string }[] = [
  { id: "all", label: "Tous" },
  { id: "administratif", label: "Administratif" },
  { id: "direction", label: "Direction" },
  { id: "professeur", label: "Professeur" },
  { id: "autre", label: "Autre" },
];

const DIRECTION_SET = new Set<string>(INTRANET_DIRECTION_SLUGS);

export function escortDisplayName(u: TravelsEscortDirectoryUser): string {
  const composed = `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim();
  return (u.displayName || composed || u.email).trim();
}

export function escortMatchesRoleFilter(
  roles: string[],
  filter: TravelsEscortRoleFilter,
): boolean {
  if (filter === "all" || filter === "autre") return true;
  if (filter === "administratif") return roles.includes("administratif");
  if (filter === "professeur") return roles.includes("professeur");
  if (filter === "direction") return roles.some((r) => DIRECTION_SET.has(r));
  return true;
}

export function serializeAccompagnateursNames(items: TravelsAccompagnateur[]): string {
  return items
    .map((a) => a.name.trim())
    .filter(Boolean)
    .join(", ");
}

export function parseAccompagnateursFromTrip(opts: {
  nomsAccompagnateurs?: string | string[] | null;
  accompagnateurs?: TravelsAccompagnateur[] | null;
}): TravelsAccompagnateur[] {
  if (Array.isArray(opts.accompagnateurs) && opts.accompagnateurs.length > 0) {
    return opts.accompagnateurs
      .map((a) => ({
        userId: a.userId?.trim() || undefined,
        name: String(a.name || "").trim(),
        email: a.email?.trim() || undefined,
        source: a.source === "directory" ? ("directory" as const) : ("autre" as const),
      }))
      .filter((a) => a.name);
  }

  const raw = opts.nomsAccompagnateurs;
  const text = Array.isArray(raw)
    ? raw.map(String).join(", ")
    : String(raw || "");
  return text
    .split(/[,;/]+/)
    .map((s) => s.trim())
    .filter(Boolean)
    .map((name) => ({ name, source: "autre" as const }));
}

/** Rematch legacy free-text names against directory when possible. */
export function hydrateAccompagnateursFromDirectory(
  items: TravelsAccompagnateur[],
  directory: TravelsEscortDirectoryUser[],
): TravelsAccompagnateur[] {
  if (!directory.length) return items;
  const byId = new Map(directory.map((u) => [u.externalUserId, u]));
  const byEmail = new Map(directory.map((u) => [u.email.trim().toLowerCase(), u]));
  const byName = new Map(
    directory.map((u) => [escortDisplayName(u).toLowerCase(), u]),
  );

  return items.map((item) => {
    if (item.userId && byId.has(item.userId)) {
      const u = byId.get(item.userId)!;
      return {
        userId: u.externalUserId,
        name: escortDisplayName(u),
        email: u.email,
        source: "directory" as const,
      };
    }
    if (item.email) {
      const u = byEmail.get(item.email.toLowerCase());
      if (u) {
        return {
          userId: u.externalUserId,
          name: escortDisplayName(u),
          email: u.email,
          source: "directory" as const,
        };
      }
    }
    const u = byName.get(item.name.toLowerCase());
    if (u) {
      return {
        userId: u.externalUserId,
        name: escortDisplayName(u),
        email: u.email,
        source: "directory" as const,
      };
    }
    return { ...item, source: item.source === "directory" ? "directory" : "autre" };
  });
}
