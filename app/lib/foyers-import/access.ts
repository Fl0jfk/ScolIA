import "server-only";

import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/intranet-session";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { canManageFoyersRoles } from "./roles";

export { canManageFoyersRoles } from "./roles";

export type FoyersGate =
  | { ok: true; etablissementId: string; actorUserId: string; roles: string[] }
  | { ok: false; response: NextResponse };

/**
 * Même niveau que les actions foyer de /api/eleves/[id]/dossier (create_foyer, add_responsable) :
 * rôle admin / direction / administratif (canEditStructure) ET section « famille » accordée.
 */
export async function requireFoyersGate(): Promise<FoyersGate> {
  const gate = await requireAppUser();
  if (!gate.ok) {
    return {
      ok: false,
      response: NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }),
    };
  }
  const roles = Array.isArray(gate.user.roles) ? gate.user.roles : [];
  const orgAdmin = Boolean(gate.user.orgAdmin);
  const platformAdmin = Boolean(gate.user.platformAdmin);
  if (!canManageFoyersRoles(roles, { orgAdmin, platformAdmin })) {
    return { ok: false, response: NextResponse.json({ error: "Non autorisé." }, { status: 403 }) };
  }
  const etablissementId = await resolveCurrentEtablissementId();
  if (!etablissementId) {
    return { ok: false, response: NextResponse.json({ error: "Établissement introuvable." }, { status: 400 }) };
  }
  const { loadModuleAccess } = await import("@/app/lib/module-access-store");
  const { dossierSectionsForRolesWithAccess } = await import("@/app/lib/module-access");
  const access = await loadModuleAccess();
  const sections = dossierSectionsForRolesWithAccess(roles, { orgAdmin, platformAdmin }, access, {
    userId: gate.user.id,
    businessUserId: gate.user.businessUserId,
  });
  if (!sections.has("famille")) {
    return { ok: false, response: NextResponse.json({ error: "Non autorisé." }, { status: 403 }) };
  }
  return { ok: true, etablissementId, actorUserId: gate.user.id, roles };
}
