import { NextResponse } from "next/server";
import { loadAllEstablishments, loadAppConfig } from "@/app/lib/app-config";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { resolveHeaderLogoDisplayUrl } from "@/app/lib/branding-logo";

export async function GET(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  try {
    const [config, allEstablishments] = await Promise.all([loadAppConfig(), loadAllEstablishments()]);
    const headerLogoPreviewUrl = await resolveHeaderLogoDisplayUrl(config.identity.headerLogoUrl);

    await writeDataAccessAudit({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      resourceType: "settings",
      action: "read",
      req,
    });

    return NextResponse.json({
      config: { ...config, establishments: allEstablishments },
      headerLogoPreviewUrl,
    });
  } catch (e) {
    console.error("[settings] GET", e);
    return NextResponse.json({ error: "Impossible de charger la configuration." }, { status: 500 });
  }
}
