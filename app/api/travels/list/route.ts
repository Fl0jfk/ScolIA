import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { compareTripsByTravelDate } from "@/app/lib/travels-trip-helpers";
import { normalizeTripImageFields } from "@/app/lib/travels-image-url";
import { listTravelsForEtablissement } from "@/app/lib/travels-storage";

export async function GET(req: Request) {
  const gate = await requireModule("travels");
  if (!gate.ok) return gate.response;

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;

  try {
    const trips = await listTravelsForEtablissement(tenant.ctx.etablissementId);
    const sortedTrips = [...trips].sort(compareTripsByTravelDate).map(normalizeTripImageFields);

    await writeDataAccessAudit({
      etablissementId: tenant.ctx.etablissementId,
      userId: tenant.ctx.authUserId,
      resourceType: "travel",
      action: "list",
      req,
      metadata: { count: sortedTrips.length },
    });

    return NextResponse.json(sortedTrips);
  } catch (error) {
    console.error("[travels/list GET]", error);
    const detail = error instanceof Error ? error.message : String(error);
    return NextResponse.json(
      {
        error: "Impossible de charger les voyages.",
        code: "TRAVELS_LIST_ERROR",
        detail,
      },
      { status: 503 },
    );
  }
}
