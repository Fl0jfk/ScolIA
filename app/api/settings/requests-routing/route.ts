import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { parseRequestsRouting } from "@/app/lib/app-config-schemas";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  getRequestsRoutingConfig,
  saveRequestsRoutingConfig,
} from "@/app/lib/requests-routing-config";

export async function GET() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const config = await getRequestsRoutingConfig();
    return NextResponse.json({ config });
  } catch (e) {
    console.error("[settings/requests-routing] GET", e);
    return NextResponse.json({ error: "Impossible de charger le routage." }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const rawConfig =
      body && typeof body === "object" && "config" in body && body.config
        ? body.config
        : body;
    const preserveTags =
      body && typeof body === "object" && "preserveTags" in body
        ? body.preserveTags !== false
        : true;
    const parsed = parseRequestsRouting(rawConfig);
    const saved = await saveRequestsRoutingConfig(parsed, { preserveTags });
    const user = await safeCurrentUser();
    return NextResponse.json({
      success: true,
      audit: {
        updatedAt: new Date().toISOString(),
        updatedBy: user?.fullName || user?.id || "admin",
      },
      config: saved,
    });
  } catch (e) {
    console.error("[settings/requests-routing] PUT", e);
    const msg = e instanceof Error ? e.message : "Erreur enregistrement";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
