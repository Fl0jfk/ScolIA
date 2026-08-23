import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  roleSlugsForEstablishment,
  directionRoleForKind,
} from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { getDirectoryUserRoles, syncDirectoryUserRoles } from "@/app/lib/directory-members";
import { hasMasterRole } from "@/app/lib/intranet-roles";

function directorAssignments(list: Establishment[]): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>();
  for (const est of list) {
    const uid = est.directorExternalUserId?.trim();
    if (!uid) continue;
    const role = directionRoleForKind(inferEstablishmentKind(est));
    const set = map.get(uid) ?? new Set<string>();
    set.add(role);
    map.set(uid, set);
  }
  return map;
}

async function applyRoles(externalUserId: string, nextRoles: string[]): Promise<void> {
  const existing = await getDirectoryUserRoles(externalUserId);
  if (hasMasterRole(existing)) return;
  const merged = [...existing];
  for (const r of nextRoles) {
    if (!merged.includes(r)) merged.push(r);
  }
  if (merged.length === 0) return;
  const same = merged.length === existing.length && merged.every((r) => existing.includes(r));
  if (same) return;
  await syncDirectoryUserRoles(externalUserId, merged);
}

async function stripRoles(externalUserId: string, remove: Set<string>): Promise<void> {
  if (remove.size === 0) return;
  const existing = await getDirectoryUserRoles(externalUserId);
  if (hasMasterRole(existing)) return;
  const next = existing.filter((r) => !remove.has(r));
  if (next.length === existing.length) return;
  if (next.length === 0) return;
  await syncDirectoryUserRoles(externalUserId, next);
}

/** Ajoute / retire les rôles direction selon les responsables d’établissement. */
export async function syncEstablishmentDirectorRoles(
  previous: Establishment[],
  next: Establishment[],
): Promise<void> {
  const before = directorAssignments(previous);
  const after = directorAssignments(next);

  const ids = new Set([...before.keys(), ...after.keys()]);
  for (const externalUserId of ids) {
    try {
      const was = before.get(externalUserId) ?? new Set<string>();
      const now = after.get(externalUserId) ?? new Set<string>();
      const toAdd = [...now].filter((r) => !was.has(r));
      const toRemove = [...was].filter((r) => !now.has(r));
      if (toAdd.length) await applyRoles(externalUserId, toAdd);
      if (toRemove.length) await stripRoles(externalUserId, new Set(toRemove));
    } catch (e) {
      console.error("[establishment-director-sync]", externalUserId, e);
    }
  }
}

export function withDerivedRoleSlugs(est: Establishment): Establishment {
  return { ...est, roleSlugs: roleSlugsForEstablishment(est) };
}
