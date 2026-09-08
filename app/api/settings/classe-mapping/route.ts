import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import { requireTenantId } from "@/app/lib/tenant-scope";
import {
  loadClasseMappingWorkspace,
  saveClasseSiteMappings,
} from "@/app/lib/classe-site-mapping";
import { invalidateEleveDossierClassCatalog } from "@/app/lib/eleve-dossier-catalog";
import { isDatabaseConfigured } from "@/db/index";

const mappingItemSchema = z.object({
  className: z.string().trim().min(1).max(120),
  siteId: z.string().trim().min(1).max(80),
  siecleCode: z.string().trim().max(80).nullable().optional(),
});

const putSchema = z.object({
  mappings: z.array(mappingItemSchema).max(800),
});

export async function GET() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;
  if (!isDatabaseConfigured()) {
    return NextResponse.json(
      { error: "Base indisponible.", code: "ENT_DB_REQUIRED" },
      { status: 503 },
    );
  }

  const workspace = await loadClasseMappingWorkspace(tenant.ctx.etablissementId);
  return NextResponse.json(workspace);
}

export async function PUT(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;
  if (!isDatabaseConfigured()) {
    return NextResponse.json(
      { error: "Base indisponible.", code: "ENT_DB_REQUIRED" },
      { status: 503 },
    );
  }

  const parsed = putSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json(
      { error: "Données de matching invalides.", code: "INVALID_MAPPING" },
      { status: 400 },
    );
  }

  const workspace = await loadClasseMappingWorkspace(tenant.ctx.etablissementId);
  const allowedSiteIds = new Set(workspace.sites.map((s) => s.siteId));

  try {
    const mappings = await saveClasseSiteMappings({
      etablissementId: tenant.ctx.etablissementId,
      mappings: parsed.data.mappings,
      allowedSiteIds,
    });
    invalidateEleveDossierClassCatalog();
    return NextResponse.json({ ok: true, mappings });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Enregistrement impossible." },
      { status: 400 },
    );
  }
}
