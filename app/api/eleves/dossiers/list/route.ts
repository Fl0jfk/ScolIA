import { NextRequest, NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import {
  canViewFullElevesDossierHub,
  isProfesseurScopedDossierViewer,
  listAssignedClassesForTeacher,
  listElevesDossierFromDb,
} from "@/app/lib/eleve-dossier-prof";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { etablissementSite } from "@/db/schema";
import { eq } from "drizzle-orm";

export async function GET(req: NextRequest) {
  const gate = await requireModule("eleve-dossier");
  if (!gate.ok) return gate.response;

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  const { searchParams } = req.nextUrl;
  const siteId = searchParams.get("siteId")?.trim() || undefined;
  const classe = searchParams.get("classe")?.trim() || undefined;
  const status = searchParams.get("status")?.trim() || undefined;

  const user = gate.ctx.user;
  const fullHub = canViewFullElevesDossierHub({
    roles: user.roles,
    orgAdmin: user.orgAdmin,
    platformAdmin: user.platformAdmin,
  });
  const profScoped = isProfesseurScopedDossierViewer({
    roles: user.roles,
    orgAdmin: user.orgAdmin,
    platformAdmin: user.platformAdmin,
  });

  let assignedClasses: string[] | undefined;
  if (profScoped) {
    assignedClasses = await listAssignedClassesForTeacher(user.businessUserId);
    if (assignedClasses.length === 0) {
      return NextResponse.json({
        eleves: [],
        assignedClasses: [],
        canViewFullHub: false,
        profScoped: true,
        sites: [],
        message:
          "Aucune classe assignée. L’administratif doit vous renseigner dans Paramètres (roster) ou Stages (référent).",
      });
    }
  } else if (!fullHub) {
    return NextResponse.json(
      { error: "Accès refusé à la liste des dossiers.", code: "DOSSIER_LIST_FORBIDDEN" },
      { status: 403 },
    );
  }

  if (!isDatabaseConfigured()) {
    return NextResponse.json(
      { error: "Liste dossiers indisponible sans base ENT.", code: "ENT_DB_REQUIRED" },
      { status: 503 },
    );
  }

  const eleves = await listElevesDossierFromDb(tenant.ctx.etablissementId, {
    siteId: fullHub ? siteId : undefined,
    classe: profScoped && classe ? classe : fullHub ? classe : undefined,
    status: fullHub ? status : undefined,
    assignedClasses: profScoped ? assignedClasses : undefined,
  });

  const db = getDb();
  const sites = await db
    .select({
      siteId: etablissementSite.siteId,
      label: etablissementSite.label,
    })
    .from(etablissementSite)
    .where(eq(etablissementSite.etablissementId, tenant.ctx.etablissementId));

  if (eleves.length > 0) {
    await writeDataAccessAudit({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      resourceType: "eleves_registry",
      action: "list",
      req,
      metadata: {
        count: eleves.length,
        profScoped,
        filters: { siteId, classe, status },
      },
    });
  }

  return NextResponse.json({
    eleves,
    assignedClasses: assignedClasses ?? [],
    canViewFullHub: fullHub,
    profScoped,
    sites,
  });
}
