import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { searchAccueilPersonnes } from "@/app/lib/accueil-absences-search";

export async function GET(req: Request) {
  const gate = await requireModule("accueil-absences");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const q = new URL(req.url).searchParams.get("q")?.trim() || "";
  if (q.length < 3) return NextResponse.json({ hits: [] });

  const hits = await searchAccueilPersonnes(etabId, q);
  return NextResponse.json({ hits });
}
