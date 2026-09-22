import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  createPassage,
  getAlertesCantinePourEleve,
  listPassagesDuJour,
  searchElevesForPassage,
} from "@/app/lib/passages-db";

export async function GET(req: Request) {
  const gate = await requireModule("passages");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim();
  if (q) {
    const eleves = await searchElevesForPassage(etabId, q);
    return NextResponse.json({ eleves });
  }

  const eleveId = url.searchParams.get("eleveId")?.trim();
  if (eleveId && url.searchParams.get("alertes") === "cantine") {
    const alertes = await getAlertesCantinePourEleve(etabId, eleveId);
    return NextResponse.json({ alertes });
  }

  const lieu = url.searchParams.get("lieu")?.trim() || undefined;
  const date = url.searchParams.get("date")?.trim() || undefined;
  const passages = await listPassagesDuJour(etabId, { lieu, date });
  return NextResponse.json({ passages });
}

export async function POST(req: Request) {
  const gate = await requireModule("passages");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = (await req.json().catch(() => ({}))) as {
    eleveId?: string;
    inviteNom?: string;
    sens?: string;
    lieu?: string;
  };

  try {
    const result = await createPassage(etabId, {
      eleveId: body.eleveId,
      inviteNom: body.inviteNom,
      sens: String(body.sens || ""),
      lieu: String(body.lieu || ""),
      source: "manuel",
    });
    return NextResponse.json({ passage: result, alertesCantine: result.alertesCantine ?? [] });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Enregistrement impossible." },
      { status: 400 },
    );
  }
}
