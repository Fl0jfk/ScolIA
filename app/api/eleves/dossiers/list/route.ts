import { NextRequest, NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import {
  canViewFullElevesDossierHub,
  isProfesseurScopedDossierViewer,
  listAssignedClassesForTeacher,
  listElevesDossierFromDb,
} from "@/app/lib/eleve-dossier-prof";
import {
  buildEleveDossierClassCatalog,
  classOptionLabel,
  dossierClassOptionsForSite,
  enrichEleveDossierListItem,
  resolveSiteIdForClass,
  resolveSiteLabel,
} from "@/app/lib/eleve-dossier-catalog";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { backfillElevesScolariteCouranteOnce } from "@/app/lib/ent-core-db";
import { resolvePhotoUrlsForEleves } from "@/app/lib/eleve-photos";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { etablissementSite } from "@/db/schema";
import { eq } from "drizzle-orm";

export async function GET(req: NextRequest) {
  try {
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

  try {
    await backfillElevesScolariteCouranteOnce(tenant.ctx.etablissementId);
  } catch (e) {
    console.warn("[eleves/dossiers/list] backfill scolarité", e);
  }

  const elevesRaw = await listElevesDossierFromDb(tenant.ctx.etablissementId, {
    classe: profScoped && classe ? classe : fullHub ? classe : undefined,
    status: fullHub ? status : undefined,
    assignedClasses: profScoped ? assignedClasses : undefined,
  });

  const db = getDb();
  const sites = await db
    .select({
      siteId: etablissementSite.siteId,
      label: etablissementSite.label,
      kind: etablissementSite.kind,
    })
    .from(etablissementSite)
    .where(eq(etablissementSite.etablissementId, tenant.ctx.etablissementId));

  const catalog = await buildEleveDossierClassCatalog(sites);

  let eleves = elevesRaw.map((row) => enrichEleveDossierListItem(row, catalog));

  if (fullHub && siteId) {
    eleves = eleves.filter((e) => e.siteId === siteId);
  }

  const photoUrls = await resolvePhotoUrlsForEleves(
    eleves.map((e) => ({
      id: e.id,
      nom: e.nom,
      prenom: e.prenom,
      ine: e.ine,
      photoKey: e.photoKey,
    })),
  );
  eleves = eleves.map((e) => ({
    ...e,
    photoUrl: photoUrls[e.id] ?? null,
    photoKey: undefined,
  }));

  const extraClasses = [
    ...new Set(eleves.map((e) => e.classe).filter((c): c is string => Boolean(c?.trim()))),
  ];
  const classOptions = profScoped
    ? (assignedClasses ?? [])
        .map((cls) => {
          const fromCatalog = catalog.classOptions.find((o) => o.value === cls);
          if (fromCatalog) return fromCatalog;
          const clsSiteId = resolveSiteIdForClass(cls, catalog);
          const clsSiteLabel = resolveSiteLabel(clsSiteId, catalog);
          return {
            value: cls,
            label: classOptionLabel(cls, clsSiteLabel),
            siteId: clsSiteId,
            siteLabel: clsSiteLabel,
          };
        })
        .sort((a, b) =>
          a.label.localeCompare(b.label, "fr", { sensitivity: "base", numeric: true }),
        )
    : dossierClassOptionsForSite(catalog, fullHub ? siteId : undefined, extraClasses);

  const siteLabelById = Object.fromEntries(catalog.siteLabelById.entries());

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
    eleves: eleves.map((e) =>
      profScoped
        ? { ...e, ine: null, sourceKey: undefined }
        : { ...e, sourceKey: undefined },
    ),
    assignedClasses: assignedClasses ?? [],
    canViewFullHub: fullHub,
    profScoped,
    sites: sites.map((s) => ({ siteId: s.siteId, label: s.label })),
    siteLabelById,
    classOptions,
  });
  } catch (error) {
    console.error("[eleves/dossiers/list]", error);
    return NextResponse.json(
      {
        error: "Impossible de charger les dossiers élèves.",
        code: "DOSSIER_LIST_ERROR",
        detail: error instanceof Error ? error.message : String(error),
      },
      { status: 500 },
    );
  }
}
