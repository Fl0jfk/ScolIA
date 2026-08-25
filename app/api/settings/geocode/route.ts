import { NextResponse } from "next/server";
import { geocodeFrenchAddress } from "@/app/lib/geocode-address";
import { requireModule } from "@/app/lib/intranet-auth";

export async function POST(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const geo = await geocodeFrenchAddress({
      street: String(body.street || ""),
      zip: String(body.zip || ""),
      city: String(body.city || ""),
    });
    if (!geo) {
      return NextResponse.json({ error: "Adresse introuvable. Vérifiez rue, code postal et ville." }, { status: 404 });
    }
    return NextResponse.json({
      latitude: geo.latitude,
      longitude: geo.longitude,
      displayName: geo.displayName,
    });
  } catch (e) {
    console.error("[settings/geocode]", e);
    return NextResponse.json({ error: "Erreur géocodage." }, { status: 500 });
  }
}
