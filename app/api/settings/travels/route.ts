import { NextResponse } from "next/server";
import { loadAppConfig, saveTravelsModule } from "@/app/lib/app-config";
import { parseTravelsModule } from "@/app/lib/app-config-schemas";
import { requireModule } from "@/app/lib/intranet-auth";

export async function GET() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  const config = await loadAppConfig();
  return NextResponse.json({ travels: config.travels });
}

export async function PUT(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    await saveTravelsModule(parseTravelsModule(body));
    const config = await loadAppConfig();
    return NextResponse.json({ success: true, travels: config.travels });
  } catch (e) {
    console.error("[settings/travels]", e);
    const msg = e instanceof Error ? e.message : "Erreur enregistrement";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
