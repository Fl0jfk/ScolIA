import { NextResponse } from "next/server";
import { loadAppConfig, saveExternalLinks } from "@/app/lib/app-config";
import { parseExternalLinksFile } from "@/app/lib/app-config-schemas";
import { requireModule } from "@/app/lib/intranet-auth";

export async function GET() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  const config = await loadAppConfig();
  return NextResponse.json({ links: config.externalLinks });
}

export async function PUT(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const links = parseExternalLinksFile(body);
    await saveExternalLinks(links);
    return NextResponse.json({ success: true, links });
  } catch (e) {
    console.error("[settings/external-links]", e);
    const msg = e instanceof Error ? e.message : "Erreur enregistrement";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
