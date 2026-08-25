import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  ensureSuggestedAnneeScolaire,
  listAnneesScolaires,
  setCurrentAnneeScolaire,
  upsertAnneeScolaire,
} from "@/app/lib/annees-scolaires-db";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  await ensureSuggestedAnneeScolaire(etabId);
  const annees = await listAnneesScolaires(etabId);
  const current = annees.find((a) => a.isCurrent) ?? null;
  return NextResponse.json({ annees, current });
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "upsert");

  try {
    if (action === "setCurrent") {
      const row = await setCurrentAnneeScolaire(etabId, String(body.anneeId || ""));
      return NextResponse.json({ ok: true, annee: row });
    }
    if (action === "upsert" || action === "openYear") {
      const row = await upsertAnneeScolaire(etabId, {
        label: String(body.label || ""),
        startsOn: body.startsOn ?? null,
        endsOn: body.endsOn ?? null,
        makeCurrent: body.makeCurrent !== false,
      });
      return NextResponse.json({ ok: true, annee: row });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur année scolaire." },
      { status: 400 },
    );
  }
}
