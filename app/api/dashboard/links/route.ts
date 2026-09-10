import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { dashboardQuickLinksFromExternalLinks } from "@/app/lib/dashboard-external-links";
import { requireAuth } from "@/app/lib/intranet-auth";
import { valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import { VALKEY_TTL, valkeyKeyDashboardLinks } from "@/app/lib/valkey-keys";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    let etabId = "default";
    try {
      const { requireTenantId } = await import("@/app/lib/tenant-scope");
      const tenant = await requireTenantId();
      if (tenant.ok) etabId = tenant.ctx.etablissementId;
    } catch {
      /* ignore */
    }

    const cacheKey = valkeyKeyDashboardLinks(etabId);
    const cached = await valkeyGetJson<{ links: unknown[] }>(cacheKey);
    if (cached?.links) {
      return NextResponse.json(cached);
    }

    const config = await loadAppConfig();
    const payload = {
      links: dashboardQuickLinksFromExternalLinks(config.externalLinks),
    };
    void valkeySetJson(cacheKey, payload, VALKEY_TTL.dashboardLinks);
    return NextResponse.json(payload);
  } catch (e) {
    console.error("[dashboard/links]", e);
    return NextResponse.json({ links: [] });
  }
}
