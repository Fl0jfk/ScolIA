import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { parisDateKey } from "@/app/lib/paris-time";
import { canSeeAccueilBoardKind } from "@/app/lib/accueil-absences-access";
import { listAccueilBoard } from "@/app/lib/accueil-absences-db";
import { ABSENCES_ACCUEIL_CONSULTATION_MODULE_ID } from "@/app/lib/accueil-absences-types";

export async function GET(req: Request) {
  const gate = await requireModule(ABSENCES_ACCUEIL_CONSULTATION_MODULE_ID);
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const rawDate = new URL(req.url).searchParams.get("date")?.trim();
  const date =
    rawDate && /^\d{4}-\d{2}-\d{2}$/.test(rawDate) ? rawDate : parisDateKey(new Date());

  const rows = await listAccueilBoard(etabId, date);
  const visible = rows.filter((r) => canSeeAccueilBoardKind(r.kind, gate.ctx.user.roles));

  return NextResponse.json({ date, rows: visible });
}
