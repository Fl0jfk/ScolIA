import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getInternatRepasSoirDuJour } from "@/app/lib/internat-repas-soir-db";

export async function GET(req: Request) {
  const gate = await requireModule("internat");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const date = new URL(req.url).searchParams.get("date")?.trim() || undefined;
  try {
    const payload = await getInternatRepasSoirDuJour(etabId, { date });
    return NextResponse.json(payload);
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Chargement impossible." },
      { status: 400 },
    );
  }
}
