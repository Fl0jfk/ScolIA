import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { requireAppUser } from "@/app/lib/app-session";
import {
  countAbsencesATraiter,
  enrichAbsencesWithInternat,
  listAbsencesATraiter,
  markAbsenceRelance,
  updateAbsenceCpe,
  type AbsenceStatut,
} from "@/app/lib/vs-absences-db";

export async function GET(req: Request) {
  const gate = await requireModule("vs-absences");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const statut = (url.searchParams.get("statut")?.trim() || "a_traiter") as AbsenceStatut;
  const [rawAbsences, aTraiter] = await Promise.all([
    listAbsencesATraiter(etabId, { statut, limit: 200 }),
    countAbsencesATraiter(etabId),
  ]);
  const absences = await enrichAbsencesWithInternat(rawAbsences);

  return NextResponse.json({ absences, counts: { aTraiter } });
}

export async function PATCH(req: Request) {
  const gate = await requireModule("vs-absences");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    id?: string;
    ids?: string[];
    statut?: AbsenceStatut;
    justifie?: boolean;
    motif?: string;
    noteCpe?: string;
  };

  if (body.action === "relance") {
    const ids = Array.isArray(body.ids) ? body.ids.filter(Boolean) : [];
    if (!ids.length) return NextResponse.json({ error: "ids requis." }, { status: 400 });
    const n = await markAbsenceRelance(etabId, ids);
    return NextResponse.json({ ok: true, updated: n });
  }

  const id = String(body.id || "").trim();
  if (!id) return NextResponse.json({ error: "id requis." }, { status: 400 });

  const row = await updateAbsenceCpe(etabId, id, {
    statut: body.statut,
    justifie: body.justifie,
    motif: body.motif,
    noteCpe: body.noteCpe,
    traiteParUserId: appUser.ok ? appUser.user.id : null,
  });
  if (!row) return NextResponse.json({ error: "Absence introuvable." }, { status: 404 });
  return NextResponse.json({ absence: row });
}
