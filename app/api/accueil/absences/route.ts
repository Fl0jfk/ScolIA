import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { parisDateKey } from "@/app/lib/paris-time";
import { canSeeAccueilBoardKind } from "@/app/lib/accueil-absences-access";
import {
  cancelAccueilAbsence,
  declareAccueilAbsence,
  listAccueilBoard,
} from "@/app/lib/accueil-absences-db";

const DeclareSchema = z.object({
  kind: z.enum(["eleve", "enseignant", "personnel"]),
  subjectId: z.string().min(1),
  mode: z.enum(["today", "hours", "multi_day"]),
  startDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  endDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).optional(),
  startTime: z.string().regex(/^\d{1,2}:\d{2}$/).optional().nullable(),
  endTime: z.string().regex(/^\d{1,2}:\d{2}$/).optional().nullable(),
  motif: z.string().max(400).optional().nullable(),
  canal: z.enum(["telephone", "physique", "mail"]).optional(),
  eleveNature: z.enum(["absence", "retard"]).optional(),
});

const CancelSchema = z.object({
  action: z.literal("annuler"),
  id: z.string().min(1),
});

export async function GET(req: Request) {
  const gate = await requireModule("accueil-absences");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const date = new URL(req.url).searchParams.get("date")?.trim() || parisDateKey(new Date());
  const rows = await listAccueilBoard(etabId, date);
  const visible = rows.filter((r) => canSeeAccueilBoardKind(r.kind, gate.ctx.user.roles));
  return NextResponse.json({ date, rows: visible, canDeclare: true });
}

export async function POST(req: Request) {
  const gate = await requireModule("accueil-absences");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const parsed = DeclareSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }
  const body = parsed.data;
  const startDate = body.startDate;
  const endDate = body.mode === "multi_day" ? body.endDate || body.startDate : body.startDate;

  try {
    const created = await declareAccueilAbsence(etabId, {
      kind: body.kind,
      subjectId: body.subjectId,
      mode: body.mode,
      startDate,
      endDate,
      startTime: body.startTime,
      endTime: body.endTime,
      motif: body.motif,
      canal: body.canal || "telephone",
      eleveNature: body.kind === "eleve" ? body.eleveNature || "absence" : undefined,
      actor: {
        userId: gate.ctx.user.id,
        name:
          [gate.ctx.user.firstName, gate.ctx.user.lastName].filter(Boolean).join(" ") ||
          gate.ctx.user.name ||
          "Accueil",
        email: gate.ctx.user.email || "",
        roles: gate.ctx.user.roles,
      },
    });
    return NextResponse.json({ ok: true, ...created });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Enregistrement impossible.";
    return NextResponse.json({ error: message }, { status: 400 });
  }
}

export async function PATCH(req: Request) {
  const gate = await requireModule("accueil-absences");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const parsed = CancelSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  try {
    const actorName =
      [gate.ctx.user.firstName, gate.ctx.user.lastName].filter(Boolean).join(" ") ||
      gate.ctx.user.name ||
      "Accueil";
    const ok = await cancelAccueilAbsence(etabId, parsed.data.id, actorName);
    if (!ok) return NextResponse.json({ error: "Absence introuvable." }, { status: 404 });
    return NextResponse.json({ ok: true });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Annulation impossible.";
    return NextResponse.json({ error: message }, { status: 400 });
  }
}
