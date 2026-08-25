import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  deleteCalendrierEntry,
  deleteEdtCreneau,
  listCalendrierEntries,
  listEdtCreneaux,
  seedDefaultCalendrier,
  upsertCalendrierEntry,
  upsertEdtCreneau,
} from "@/app/lib/vs-calendrier-db";
import { listGroupes } from "@/app/lib/groupes-pedagogiques-db";

export async function GET(req: Request) {
  const gate = await requireModule("vs-calendrier");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const classe = url.searchParams.get("classe")?.trim() || undefined;
  const groupeId = url.searchParams.get("groupeId")?.trim() || undefined;

  const [calendrier, creneaux, groupes, anneeCourante] = await Promise.all([
    listCalendrierEntries(etabId),
    listEdtCreneaux(etabId, { classe, groupeId }),
    listGroupes(etabId),
    import("@/app/lib/annees-scolaires-db").then((m) => m.resolveAnneeCouranteMeta(etabId)),
  ]);

  const { detectEdtConflicts } = await import("@/app/lib/edt-conflicts");
  const conflits = detectEdtConflicts(
    creneaux.map((c) => ({
      id: c.id,
      jourSemaine: c.jourSemaine,
      heureDebut: c.heureDebut,
      heureFin: c.heureFin,
      classe: c.classe,
      groupeId: c.groupeId,
      groupeCode: c.groupeCode,
      enseignantNom: c.enseignantNom,
      salle: c.salle,
      semaine: c.semaine,
    })),
  );

  return NextResponse.json({ calendrier, creneaux, groupes, anneeCourante, conflits });
}

export async function POST(req: Request) {
  const gate = await requireModule("vs-calendrier");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => null);
  const action = String(body?.action || "");

  try {
    if (action === "seedDefaults") {
      const result = await seedDefaultCalendrier(etabId);
      return NextResponse.json(result);
    }

    if (action === "upsertCalendrier") {
      const row = await upsertCalendrierEntry(etabId, body.entry || body);
      return NextResponse.json({ ok: true, row });
    }

    if (action === "deleteCalendrier") {
      const id = String(body?.id || "");
      if (!id) return NextResponse.json({ error: "id requis." }, { status: 400 });
      await deleteCalendrierEntry(etabId, id);
      return NextResponse.json({ ok: true });
    }

    if (action === "upsertCreneau") {
      const payload = body.creneau || body.entry || body;
      const row = await upsertEdtCreneau(etabId, {
        ...payload,
        force: Boolean(body.force || payload.force),
      });
      return NextResponse.json({ ok: true, row });
    }

    if (action === "deleteCreneau") {
      const id = String(body?.id || "");
      if (!id) return NextResponse.json({ error: "id requis." }, { status: 400 });
      await deleteEdtCreneau(etabId, id);
      return NextResponse.json({ ok: true });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Opération impossible." },
      { status: 400 },
    );
  }
}
