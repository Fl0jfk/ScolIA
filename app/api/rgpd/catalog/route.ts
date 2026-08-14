import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { RGPD_CATALOG } from "@/app/lib/rgpd-catalog";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { buildDocumentList, computeRgpdComplianceScore } from "@/app/lib/rgpd-scoring";
import { loadRgpdWorkspace } from "@/app/lib/rgpd-storage";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessRgpdModule(roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const workspace = await loadRgpdWorkspace();
  return NextResponse.json({
    catalog: RGPD_CATALOG,
    documents: buildDocumentList(workspace),
    score: computeRgpdComplianceScore(workspace),
  });
}
