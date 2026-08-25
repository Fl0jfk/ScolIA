import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { requireAppUser } from "@/app/lib/app-session";
import {
  accessibleModuleIdsForRoles,
  dossierSectionsForRolesWithAccess,
} from "@/app/lib/module-access";
import { loadModuleAccess } from "@/app/lib/module-access-store";

/** Modules + sections dossier effectifs pour l’utilisateur courant (dashboard / hubs). */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }

  const access = await loadModuleAccess();
  const isOrgAdmin = isOrgAdminFromAppUser(appUser.user);
  const lookup = {
    userId: appUser.user.id,
    businessUserId: appUser.user.businessUserId,
  };
  const moduleIds = [
    ...accessibleModuleIdsForRoles(appUser.user.roles, isOrgAdmin, access, lookup),
  ];
  const dossierSections = [
    ...dossierSectionsForRolesWithAccess(
      appUser.user.roles,
      { orgAdmin: isOrgAdmin, platformAdmin: appUser.user.platformAdmin },
      access,
      lookup,
    ),
  ];

  return NextResponse.json({ moduleIds, dossierSections });
}
