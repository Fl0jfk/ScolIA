import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { requireAppUser } from "@/app/lib/app-session";
import {
  accessibleModuleIdsForRoles,
  dossierSectionsForRolesWithAccess,
} from "@/app/lib/module-access";
import { loadModuleAccess } from "@/app/lib/module-access-store";
import { isOrgAdminFromPublicMetadata, safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

/** Modules + sections dossier effectifs pour l’utilisateur courant (dashboard / hubs). */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const access = await loadModuleAccess();
    const appUser = await requireAppUser();

    if (appUser.ok) {
      const isOrgAdmin = isOrgAdminFromAppUser(appUser.user);
      const lookup = {
        userId: appUser.user.id,
        businessUserId: appUser.user.businessUserId,
      };
      const moduleIds = [
        ...accessibleModuleIdsForRoles(appUser.user.roles, isOrgAdmin, access, lookup),
      ];
      try {
        const { loadAppConfig } = await import("@/app/lib/app-config");
        const { resolvePhotocopiesOpsEmails } = await import("@/app/lib/photocopies-couleur-ops");
        const { isPhotocopiesOpsHandlerResolved } = await import(
          "@/app/lib/photocopies-couleur-ops-server"
        );
        const bundle = await loadAppConfig();
        const ops = resolvePhotocopiesOpsEmails(bundle.notifications);
        if (
          isPhotocopiesOpsHandlerResolved({
            email: appUser.user.email,
            opsEmails: ops,
            moduleAccess: access,
            lookup,
            roles: appUser.user.roles,
          }) &&
          !moduleIds.includes("photocopies-couleur")
        ) {
          moduleIds.push("photocopies-couleur");
        }
      } catch {
        /* ignore */
      }
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

    // Repli session compat : évite un dashboard sans aucun module si requireAppUser échoue.
    const user = await safeCurrentUser();
    if (!user) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
    }
    const roles = intranetRolesFromMetadata(user.publicMetadata);
    const isOrgAdmin = isOrgAdminFromPublicMetadata(user.publicMetadata);
    const lookup = {
      userId: null as string | null,
      businessUserId: user.id,
    };
    const moduleIds = [...accessibleModuleIdsForRoles(roles, isOrgAdmin, access, lookup)];
    const dossierSections = [
      ...dossierSectionsForRolesWithAccess(
        roles,
        { orgAdmin: isOrgAdmin, platformAdmin: false },
        access,
        lookup,
      ),
    ];
    return NextResponse.json({ moduleIds, dossierSections, degraded: true });
  } catch (err) {
    console.error("[me/module-access]", err);
    return NextResponse.json(
      { error: "Impossible de charger les accès modules." },
      { status: 500 },
    );
  }
}
