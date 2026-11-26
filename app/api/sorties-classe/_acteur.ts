import { requireModule } from "@/app/lib/intranet-auth";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";
import type { Acteur } from "@/app/lib/sorties-classe";

/** Utilisateur connecté, vu par les règles de droits des autorisations de sortie. */
export async function requireActeurSorties() {
  const gate = await requireModule("sorties-classe");
  if (!gate.ok) return { ok: false as const, response: gate.response };
  const user = gate.ctx.user;
  const roles = normalizeIntranetRoles(user.roles);
  const acteur: Acteur = {
    userId: user.businessUserId || user.id,
    email: user.email,
    roles,
    isAdmin: isOrgAdminFromAppUser(user) || hasGlobalAdminRole(user.roles) || hasMasterRole(user.roles),
  };
  const nom = user.name || `${user.firstName ?? ""} ${user.lastName ?? ""}`.trim() || user.email;
  return { ok: true as const, acteur, nom };
}
