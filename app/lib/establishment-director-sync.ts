import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  clerkRoleSlugsForEstablishment,
  directionRoleForKind,
} from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { getClerkUserRoles, syncClerkUserRoles } from "@/app/lib/clerk-users";
import { hasMasterRole } from "@/app/lib/intranet-roles";

function directorAssignments(list: Establishment[]): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>();
  for (const est of list) {
    const uid = est.directorClerkUserId?.trim();
    if (!uid) continue;
    const role = directionRoleForKind(inferEstablishmentKind(est));
    const set = map.get(uid) ?? new Set<string>();
    set.add(role);
    map.set(uid, set);
  }
  return map;
}

async function applyRoles(clerkUserId: string, nextRoles: string[]): Promise<void> {
  const existing = await getClerkUserRoles(clerkUserId);
  if (hasMasterRole(existing)) return;
  const merged = [...existing];
  for (const r of nextRoles) {
    if (!merged.includes(r)) merged.push(r);
  }
  if (merged.length === 0) return;
  const same = merged.length === existing.length && merged.every((r) => existing.includes(r));
  if (same) return;
  await syncClerkUserRoles(clerkUserId, merged);
}

async function stripRoles(clerkUserId: string, remove: Set<string>): Promise<void> {
  if (remove.size === 0) return;
  const existing = await getClerkUserRoles(clerkUserId);
  if (hasMasterRole(existing)) return;
  const next = existing.filter((r) => !remove.has(r));
  if (next.length === existing.length) return;
  if (next.length === 0) return;
  await syncClerkUserRoles(clerkUserId, next);
}

/** Ajoute / retire les rôles direction Clerk selon les responsables d’établissement. */
export async function syncEstablishmentDirectorRoles(
  previous: Establishment[],
  next: Establishment[],
): Promise<void> {
  const before = directorAssignments(previous);
  const after = directorAssignments(next);

  const ids = new Set([...before.keys(), ...after.keys()]);
  for (const clerkUserId of ids) {
    try {
      const was = before.get(clerkUserId) ?? new Set<string>();
      const now = after.get(clerkUserId) ?? new Set<string>();
      const toAdd = [...now].filter((r) => !was.has(r));
      const toRemove = [...was].filter((r) => !now.has(r));
      if (toAdd.length) await applyRoles(clerkUserId, toAdd);
      if (toRemove.length) await stripRoles(clerkUserId, new Set(toRemove));
    } catch (e) {
      console.error("[establishment-director-sync]", clerkUserId, e);
    }
  }
}

export function withDerivedClerkRoleSlugs(est: Establishment): Establishment {
  return { ...est, clerkRoleSlugs: clerkRoleSlugsForEstablishment(est) };
}
