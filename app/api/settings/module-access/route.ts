import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { isDatabaseConfigured } from "@/db/index";
import { getTenant } from "@/app/lib/tenant-context";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { listMembersFromDb } from "@/app/lib/members-db";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import {
  defaultDossierSectionsForRole,
  defaultDossierSectionsForRoles,
  defaultModulesForRole,
  defaultModulesForRoles,
  modulesGroupedByPillar,
  parseModuleAccess,
  roleOptionsForModuleAccess,
  type ModuleAccessConfig,
} from "@/app/lib/module-access";
import { loadModuleAccess, saveModuleAccess } from "@/app/lib/module-access-store";

export async function GET() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;

  try {
    const config = await loadModuleAccess();
    const roles = roleOptionsForModuleAccess();
    const defaults: Record<string, { modules: string[]; dossierSections: string[] }> = {};
    for (const r of roles) {
      defaults[r.slug] = {
        modules: defaultModulesForRole(r.slug),
        dossierSections: defaultDossierSectionsForRole(r.slug),
      };
    }

    const tenant = await getTenant();
    const etablissementId = isDatabaseConfigured()
      ? await ensureEtablissementFromTenant(tenant)
      : null;
    const members = etablissementId
      ? await listMembersFromDb(etablissementId)
      : await listDirectoryMembers();

    const staff = members
      .filter((m) => !m.roles.includes("parent") && !m.roles.includes("eleve"))
      .map((m) => ({
        userId: m.userId || m.externalUserId,
        externalUserId: m.externalUserId,
        email: m.email,
        displayName: m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim() || m.email,
        roles: m.roles,
        baselineModules: defaultModulesForRoles(m.roles),
        baselineDossierSections: defaultDossierSectionsForRoles(m.roles),
      }));

    return NextResponse.json({
      config,
      roles,
      pillars: modulesGroupedByPillar(),
      defaults,
      members: staff,
    });
  } catch (e) {
    console.error("[module-access] GET", e);
    return NextResponse.json({ error: "Impossible de charger les droits modules." }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;

  try {
    const body = (await req.json()) as {
      config?: unknown;
      resetRole?: string;
      resetUser?: string;
    };
    if (body.resetUser) {
      const current = await loadModuleAccess();
      const next: ModuleAccessConfig = {
        byRole: { ...current.byRole },
        byUser: { ...current.byUser },
      };
      delete next.byUser[body.resetUser];
      const saved = await saveModuleAccess(next);
      return NextResponse.json({ ok: true, config: saved });
    }
    if (body.resetRole) {
      const current = await loadModuleAccess();
      const next: ModuleAccessConfig = {
        byRole: { ...current.byRole },
        byUser: { ...current.byUser },
      };
      delete next.byRole[body.resetRole];
      const saved = await saveModuleAccess(next);
      return NextResponse.json({ ok: true, config: saved });
    }
    const parsed = parseModuleAccess(body.config ?? body);
    const saved = await saveModuleAccess(parsed);
    return NextResponse.json({ ok: true, config: saved });
  } catch (e) {
    console.error("[module-access] PUT", e);
    return NextResponse.json({ error: "Enregistrement impossible." }, { status: 500 });
  }
}
