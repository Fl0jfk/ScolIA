import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { parisDateKey } from "@/app/lib/paris-time";
import { canSeeAccueilBoardKind } from "@/app/lib/accueil-absences-access";
import { cancelAccueilAbsence, listAccueilBoard } from "@/app/lib/accueil-absences-db";
import { ABSENCES_ACCUEIL_CONSULTATION_MODULE_ID } from "@/app/lib/accueil-absences-types";

const CancelSchema = z.object({
  action: z.literal("annuler"),
  id: z.string().min(1),
});

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

export async function PATCH(req: Request) {
  const gate = await requireModule(ABSENCES_ACCUEIL_CONSULTATION_MODULE_ID);
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const parsed = CancelSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  if (!canSeeAccueilBoardKind("eleve", gate.ctx.user.roles)) {
    return NextResponse.json({ error: "Droit insuffisant." }, { status: 403 });
  }

  const actorName =
    [gate.ctx.user.firstName, gate.ctx.user.lastName].filter(Boolean).join(" ") ||
    gate.ctx.user.name ||
    "Vie scolaire";

  try {
    const ok = await cancelAccueilAbsence(
      etabId,
      parsed.data.id,
      actorName,
      `Annulée depuis la consultation vie scolaire (${actorName})`,
    );
    if (!ok) return NextResponse.json({ error: "Absence introuvable." }, { status: 404 });
    return NextResponse.json({ ok: true });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Annulation impossible.";
    return NextResponse.json({ error: message }, { status: 400 });
  }
}
