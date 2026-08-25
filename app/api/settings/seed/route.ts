import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { seedAppSettingsFromDefaults, seedAppSettingsFromLaProvidence } from "@/app/lib/app-config";

export async function POST(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const url = new URL(req.url);
    const profile = url.searchParams.get("profile");
    if (profile === "laprovidence") {
      await seedAppSettingsFromLaProvidence();
    } else {
      await seedAppSettingsFromDefaults();
    }
    return NextResponse.json({ success: true });
  } catch (e) {
    console.error("[settings/seed]", e);
    return NextResponse.json({ error: "Échec initialisation configuration." }, { status: 500 });
  }
}
