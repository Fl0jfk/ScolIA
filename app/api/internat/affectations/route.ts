import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  closeInternatAffectation,
  createInternatChambre,
  listInternatAffectationsOuvertes,
  listInternatBatiments,
  listInternatChambres,
  openInternatAffectation,
} from "@/app/lib/internat-chambres-db";
import { searchElevesForPassage } from "@/app/lib/passages-db";

export async function GET(req: Request) {
  const gate = await requireModule("internat");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim();
  if (q) {
    const eleves = await searchElevesForPassage(etabId, q);
    return NextResponse.json({ eleves });
  }

  const [batiments, chambres, affectations] = await Promise.all([
    listInternatBatiments(etabId),
    listInternatChambres(etabId),
    listInternatAffectationsOuvertes(etabId),
  ]);
  return NextResponse.json({ batiments, chambres, affectations });
}

export async function POST(req: Request) {
  const gate = await requireModule("internat");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    batimentId?: string;
    batimentLabel?: string;
    label?: string;
    capacite?: number;
    etage?: string;
    aile?: string;
    chambreId?: string;
    eleveId?: string;
    dateDebut?: string;
    affectationId?: string;
    dateFin?: string;
  };

  try {
    if (body.action === "createChambre") {
      const chambre = await createInternatChambre(etabId, {
        batimentId: body.batimentId,
        batimentLabel: body.batimentLabel,
        label: String(body.label || ""),
        capacite: body.capacite,
        etage: body.etage,
        aile: body.aile,
      });
      return NextResponse.json({ chambre });
    }

    if (body.action === "openAffectation") {
      const affectation = await openInternatAffectation(etabId, {
        chambreId: String(body.chambreId || ""),
        eleveId: String(body.eleveId || ""),
        dateDebut: body.dateDebut,
      });
      return NextResponse.json({ affectation });
    }

    if (body.action === "closeAffectation") {
      await closeInternatAffectation(etabId, {
        affectationId: String(body.affectationId || ""),
        dateFin: body.dateFin,
      });
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
