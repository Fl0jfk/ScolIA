import { NextResponse } from "next/server";
import { requireActeurSorties } from "@/app/api/sorties-classe/_acteur";
import { peutCreer } from "@/app/lib/sorties-classe";
import { listerInternesPourSelection } from "@/app/lib/sorties-classe-store";

export async function GET() {
  const access = await requireActeurSorties();
  if (!access.ok) return access.response;
  if (!peutCreer(access.acteur)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }
  return NextResponse.json({ internes: await listerInternesPourSelection() });
}
