import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { requireViewUser } from "@/app/lib/app-session";
import {
  accessibleModuleIdsForRoles,
  dossierSectionsForRolesWithAccess,
} from "@/app/lib/module-access";
import { loadModuleAccess } from "@/app/lib/module-access-store";
import { isOrgAdminFromPublicMetadata, safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

type MeModuleAccessPayload = {
  moduleIds: string[];
  dossierSections: string[];
};

const meAccessL1 = new Map<string, { at: number; payload: MeModuleAccessPayload }>();
const ME_ACCESS_L1_MS = 45_000;

/** Modules + sections dossier effectifs pour l’utilisateur courant (dashboard / hubs). */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const appUserEarly = await requireViewUser();
    if (appUserEarly.ok) {
      const l1Hit = meAccessL1.get(appUserEarly.user.id);
      if (l1Hit && Date.now() - l1Hit.at < ME_ACCESS_L1_MS) {
        return NextResponse.json(l1Hit.payload);
      }

      const etab = appUserEarly.user.etablissementId?.trim() || "default";
      const { isValkeyConfigured, valkeyGetJson, valkeySetJson } = await import(
        "@/app/lib/valkey"
      );
      const { VALKEY_TTL, valkeyKeyModuleAccessUser } = await import(
        "@/app/lib/valkey-keys"
      );
      const cacheKey = valkeyKeyModuleAccessUser(etab, appUserEarly.user.id);
      const cached = isValkeyConfigured()
        ? await valkeyGetJson<MeModuleAccessPayload>(cacheKey)
        : null;
      if (cached?.moduleIds) {
        meAccessL1.set(appUserEarly.user.id, { at: Date.now(), payload: cached });
        return NextResponse.json(cached);
      }

      const access = await loadModuleAccess();
      const isOrgAdmin = isOrgAdminFromAppUser(appUserEarly.user);
      const lookup = {
        userId: appUserEarly.user.id,
        businessUserId: appUserEarly.user.businessUserId,
      };
      const moduleIds = [
        ...accessibleModuleIdsForRoles(
          appUserEarly.user.roles,
          isOrgAdmin,
          access,
          lookup,
        ),
      ];
      try {
        const { loadAppConfig } = await import("@/app/lib/app-config");
        const { resolvePhotocopiesOpsEmails } = await import(
          "@/app/lib/photocopies-couleur-ops"
        );
        const { isPhotocopiesOpsHandlerResolved } = await import(
          "@/app/lib/photocopies-couleur-ops-server"
        );
        const bundle = await loadAppConfig();
        const ops = resolvePhotocopiesOpsEmails(bundle.notifications);
        if (
          isPhotocopiesOpsHandlerResolved({
            email: appUserEarly.user.email,
            opsEmails: ops,
            moduleAccess: access,
            lookup,
            roles: appUserEarly.user.roles,
          }) &&
          !moduleIds.includes("photocopies-couleur")
        ) {
          moduleIds.push("photocopies-couleur");
        }
      } catch {
        /* ignore */
      }
      try {
        const {
          isPortesOuvertesToolEnabled,
          withoutAccueilPortesOuvertesIfDisabled,
        } = await import("@/app/lib/toolbox-config");
        const poOn = await isPortesOuvertesToolEnabled();
        const filtered = withoutAccueilPortesOuvertesIfDisabled(moduleIds, poOn);
        moduleIds.length = 0;
        moduleIds.push(...filtered);
      } catch {
        /* ignore */
      }
      const dossierSections = [
        ...dossierSectionsForRolesWithAccess(
          appUserEarly.user.roles,
          {
            orgAdmin: isOrgAdmin,
            platformAdmin: appUserEarly.user.platformAdmin,
          },
          access,
          lookup,
        ),
      ];
      const payload = { moduleIds, dossierSections };
      meAccessL1.set(appUserEarly.user.id, { at: Date.now(), payload });
      if (isValkeyConfigured()) {
        void valkeySetJson(cacheKey, payload, VALKEY_TTL.moduleAccessUser);
      }
      return NextResponse.json(payload);
    }

    const access = await loadModuleAccess();

    // Repli session compat : évite un dashboard sans aucun module si requireViewUser échoue.
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
    try {
      const {
        isPortesOuvertesToolEnabled,
        withoutAccueilPortesOuvertesIfDisabled,
      } = await import("@/app/lib/toolbox-config");
      const poOn = await isPortesOuvertesToolEnabled();
      const filtered = withoutAccueilPortesOuvertesIfDisabled(moduleIds, poOn);
      moduleIds.length = 0;
      moduleIds.push(...filtered);
    } catch {
      /* ignore */
    }
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
