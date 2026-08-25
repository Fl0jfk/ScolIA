import { NextResponse } from "next/server";
import { markOnboardingComplete } from "@/app/lib/app-config";
import { requireModule } from "@/app/lib/intranet-auth";

export async function PUT() {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    await markOnboardingComplete();
    return NextResponse.json({ success: true });
  } catch (e) {
    console.error("[onboarding/complete]", e);
    return NextResponse.json({ error: "Impossible de finaliser la configuration." }, { status: 500 });
  }
}
