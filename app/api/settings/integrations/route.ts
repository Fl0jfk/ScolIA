import { NextResponse } from "next/server";
import { loadAppConfig, saveIntegrations } from "@/app/lib/app-config";
import { parseIntegrations } from "@/app/lib/app-config-schemas";
import { requireModule } from "@/app/lib/intranet-auth";

export async function GET() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  const config = await loadAppConfig();
  return NextResponse.json({ integrations: config.integrations });
}

export async function PUT(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    await saveIntegrations(parseIntegrations(body));
    const config = await loadAppConfig();
    return NextResponse.json({ success: true, integrations: config.integrations });
  } catch (e) {
    console.error("[settings/integrations]", e);
    const msg = e instanceof Error ? e.message : "Erreur enregistrement";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
