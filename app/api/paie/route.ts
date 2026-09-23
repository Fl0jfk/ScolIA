import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  createPaieElement,
  createPaiePeriode,
  figerPaiePeriode,
  loadPaieHub,
} from "@/app/lib/paie-etablissement-db";

export async function GET(req: Request) {
  const gate = await requireModule("paie-etablissement");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const periodeId = url.searchParams.get("periodeId") || undefined;
  const data = await loadPaieHub(etabId, periodeId);
  return NextResponse.json(data);
}

export async function POST(req: Request) {
  const gate = await requireModule("paie-etablissement");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "");

  try {
    if (action === "createPeriode") {
      const row = await createPaiePeriode(etabId, {
        label: String(body.label || ""),
        dateDebut: String(body.dateDebut || ""),
        dateFin: String(body.dateFin || ""),
      });
      return NextResponse.json({ ok: true, periode: row });
    }
    if (action === "figerPeriode") {
      const row = await figerPaiePeriode(etabId, String(body.periodeId || ""));
      return NextResponse.json({ ok: true, periode: row });
    }
    if (action === "createElement") {
      const row = await createPaieElement(etabId, {
        periodeId: String(body.periodeId || ""),
        personnelId: String(body.personnelId || ""),
        nature: String(body.nature || "heure"),
        libelle: String(body.libelle || ""),
        quantite: body.quantite,
        montant: body.montant,
        absenceId: body.absenceId ? String(body.absenceId) : null,
      });
      return NextResponse.json({ ok: true, element: row });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur paie." },
      { status: 400 },
    );
  }
}
