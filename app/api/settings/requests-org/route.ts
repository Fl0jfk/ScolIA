import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";
import { parseRequestsOrg } from "@/app/lib/app-config-schemas";
import { requireModule } from "@/app/lib/intranet-auth";
import { getRequestsOrgConfig, saveRequestsOrgConfig } from "@/app/lib/requests-org-config";

export async function GET() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const config = await getRequestsOrgConfig();
    return NextResponse.json({ config });
  } catch (e) {
    console.error("[settings/requests-org] GET", e);
    return NextResponse.json({ error: "Impossible de charger l'organisation des services." }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const parsed = parseRequestsOrg(body);
    await saveRequestsOrgConfig(parsed);
    const user = await safeCurrentUser();
    return NextResponse.json({
      success: true,
      audit: {
        updatedAt: new Date().toISOString(),
        updatedBy: user?.fullName || user?.id || "admin",
      },
      config: parsed,
    });
  } catch (e) {
    console.error("[settings/requests-org] PUT", e);
    const msg = e instanceof Error ? e.message : "Erreur enregistrement";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
