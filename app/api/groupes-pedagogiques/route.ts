import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { resolveAnneeCouranteMeta } from "@/app/lib/annees-scolaires-db";
import {
  addGroupeMembre,
  addGroupeMembresFromClasse,
  deleteGroupe,
  listGroupeMembres,
  listGroupes,
  removeGroupeMembre,
  searchElevesForGroupe,
  upsertGroupe,
} from "@/app/lib/groupes-pedagogiques-db";

export async function GET(req: Request) {
  const gate = await requireModule("groupes-pedagogiques");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const groupeId = url.searchParams.get("groupeId")?.trim();
  const search = url.searchParams.get("search")?.trim();
  const anneeCourante = await resolveAnneeCouranteMeta(etabId);

  if (search) {
    const eleves = await searchElevesForGroupe(etabId, search);
    return NextResponse.json({ eleves, anneeCourante });
  }

  if (groupeId) {
    const [groupes, membres] = await Promise.all([
      listGroupes(etabId),
      listGroupeMembres(etabId, groupeId),
    ]);
    const groupe = groupes.find((g) => g.id === groupeId) ?? null;
    return NextResponse.json({ groupe, membres, anneeCourante });
  }

  const groupes = await listGroupes(etabId);
  return NextResponse.json({ groupes, anneeCourante });
}

export async function POST(req: Request) {
  const gate = await requireModule("groupes-pedagogiques");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "upsertGroupe");

  try {
    if (action === "upsertGroupe") {
      const row = await upsertGroupe(etabId, body.groupe || body);
      return NextResponse.json({ ok: true, groupe: row });
    }

    if (action === "deleteGroupe") {
      const id = String(body.id || body.groupeId || "");
      if (!id) return NextResponse.json({ error: "id requis." }, { status: 400 });
      await deleteGroupe(etabId, id);
      return NextResponse.json({ ok: true });
    }

    if (action === "addMembre") {
      const groupeId = String(body.groupeId || "");
      const eleveId = String(body.eleveId || "");
      if (!groupeId || !eleveId) {
        return NextResponse.json({ error: "groupeId et eleveId requis." }, { status: 400 });
      }
      const result = await addGroupeMembre(etabId, groupeId, eleveId);
      return NextResponse.json({ ok: true, ...result });
    }

    if (action === "removeMembre") {
      const groupeId = String(body.groupeId || "");
      const eleveId = String(body.eleveId || "");
      if (!groupeId || !eleveId) {
        return NextResponse.json({ error: "groupeId et eleveId requis." }, { status: 400 });
      }
      await removeGroupeMembre(etabId, groupeId, eleveId);
      return NextResponse.json({ ok: true });
    }

    if (action === "addFromClasse") {
      const groupeId = String(body.groupeId || "");
      const classe = String(body.classe || "");
      if (!groupeId || !classe) {
        return NextResponse.json({ error: "groupeId et classe requis." }, { status: 400 });
      }
      const result = await addGroupeMembresFromClasse(etabId, groupeId, classe);
      return NextResponse.json({ ok: true, ...result });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Opération impossible." },
      { status: 400 },
    );
  }
}
