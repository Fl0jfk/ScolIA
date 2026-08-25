import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import {
  listFamilleAbsences,
  submitFamilleAbsenceJustification,
} from "@/app/lib/famille-absences-db";

export async function GET(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const eleveId = url.searchParams.get("eleveId")?.trim();
  const eleveIds = eleveId
    ? gate.ctx.enfants.filter((e) => e.id === eleveId).map((e) => e.id)
    : gate.ctx.enfants.map((e) => e.id);

  if (eleveId && !eleveIds.length) {
    return NextResponse.json(
      { error: "Accès refusé à cet élève.", code: "FAMILLE_ELEVE_FORBIDDEN" },
      { status: 403 },
    );
  }

  const absences = await listFamilleAbsences(gate.ctx.etablissementId, eleveIds);
  return NextResponse.json({ enfants: gate.ctx.enfants, absences });
}

export async function POST(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "justify");
  const absenceId = String(body.absenceId || "").trim();
  const motif = String(body.motif || "");

  if (action !== "justify") {
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  }
  if (!absenceId) {
    return NextResponse.json({ error: "absenceId requis." }, { status: 400 });
  }

  try {
    const row = await submitFamilleAbsenceJustification(
      gate.ctx.etablissementId,
      absenceId,
      gate.ctx.enfants.map((e) => e.id),
      motif,
    );
    if (!row) {
      return NextResponse.json(
        { error: "Absence introuvable ou non autorisée." },
        { status: 404 },
      );
    }
    return NextResponse.json({ ok: true, absence: row });
  } catch (e: unknown) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Justification impossible." },
      { status: 400 },
    );
  }
}
