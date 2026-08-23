import { NextResponse } from 'next/server';
import { requireAuth } from "@/app/lib/intranet-auth";
import { normalizeTripImageFields } from "@/app/lib/travels-image-url";
import { getTravelTrip } from "@/app/lib/travels-storage";

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { searchParams } = new URL(req.url);
  const id = searchParams.get('id');
  if (!id) return new NextResponse("ID manquant", { status: 400 });
  try {
    const trip = await getTravelTrip(id);
    if (!trip) return NextResponse.json({ error: "Impossible de récupérer le dossier" }, { status: 404 });
    return NextResponse.json(normalizeTripImageFields(trip));
  } catch (error) {
    console.error("Erreur S3 Get:", error);
    return NextResponse.json({ error: "Impossible de récupérer le dossier" }, { status: 500 });
  }
}
