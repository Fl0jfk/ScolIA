import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  closeInternatSortie,
  createInternatSortie,
  listInternatSorties,
} from "@/app/lib/internat-sorties-db";
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

  const from = url.searchParams.get("from")?.trim() || undefined;
  const to = url.searchParams.get("to")?.trim() || undefined;
  const sorties = await listInternatSorties(etabId, { from, to });
  return NextResponse.json({ sorties });
}

export async function POST(req: Request) {
  const gate = await requireModule("internat");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    eleveId?: string;
    dateDebut?: string;
    dateFin?: string;
    motif?: string;
    sortieId?: string;
  };

  try {
    if (body.action === "create") {
      const sortie = await createInternatSortie(etabId, {
        eleveId: String(body.eleveId || ""),
        dateDebut: String(body.dateDebut || ""),
        dateFin: String(body.dateFin || ""),
        motif: body.motif,
      });
      return NextResponse.json({ sortie });
    }

    if (body.action === "close") {
      await closeInternatSortie(etabId, {
        sortieId: String(body.sortieId || ""),
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
