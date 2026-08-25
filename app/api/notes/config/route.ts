import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { resolveAnneeCouranteMeta } from "@/app/lib/annees-scolaires-db";
import {
  listMatieres,
  listPeriodes,
  listTypesDevoir,
  seedNotesDefaults,
  upsertMatiere,
  upsertPeriode,
} from "@/app/lib/notes-config-db";

export async function GET() {
  const gate = await requireModule("notes");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const [matieres, periodes, typesDevoir, anneeCourante] = await Promise.all([
    listMatieres(etabId),
    listPeriodes(etabId),
    listTypesDevoir(etabId),
    resolveAnneeCouranteMeta(etabId),
  ]);
  return NextResponse.json({ matieres, periodes, typesDevoir, anneeCourante });
}

export async function POST(req: Request) {
  const gate = await requireModule("notes");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "upsertMatiere");

  try {
    if (action === "seedDefaults") {
      const result = await seedNotesDefaults(etabId);
      return NextResponse.json({ ok: true, ...result });
    }
    if (action === "upsertMatiere") {
      const row = await upsertMatiere(etabId, body);
      return NextResponse.json({ ok: true, matiere: row });
    }
    if (action === "upsertPeriode") {
      const row = await upsertPeriode(etabId, body);
      return NextResponse.json({ ok: true, periode: row });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur notes config." },
      { status: 400 },
    );
  }
}
