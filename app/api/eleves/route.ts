import { NextResponse, NextRequest } from "next/server";
import { validateElevesJson } from "@/app/lib/eleves-config";
import { requireAdmin, requireAnyModule } from "@/app/lib/intranet-auth";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import { requireTenantId } from "@/app/lib/tenant-scope";
import {
  loadElevesRegistry,
  saveElevesRegistry,
} from "@/app/lib/eleves-registry";

export async function GET(req: NextRequest) {
  try {
    const gate = await requireAnyModule(["admin-settings", "agent-ia-ocr", "pilotage-eleves"]);
    if (!gate.ok) return gate.response;

    const tenant = await requireTenantId();
    if (!tenant.ok) return tenant.response;

    const eleves = await loadElevesRegistry();
    if (eleves.length >= 1) {
      await writeDataAccessAudit({
        etablissementId: tenant.ctx.etablissementId,
        userId: tenant.ctx.authUserId,
        resourceType: "eleves_registry",
        action: "list",
        req,
        metadata: { count: eleves.length },
      });
    }
    return NextResponse.json({ count: eleves.length, eleves });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function PUT(req: NextRequest) {
  try {
    const gate = await requireAdmin();
    if (!gate.ok) return gate.response;
    const body = await req.json();
    const validated = validateElevesJson(body);
    if (!validated.ok) {
      return NextResponse.json({ error: validated.error }, { status: 400 });
    }
    await saveElevesRegistry(validated.eleves);
    return NextResponse.json({
      success: true,
      count: validated.eleves.length,
      message: `Liste mise à jour (${validated.eleves.length} élèves).`,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
