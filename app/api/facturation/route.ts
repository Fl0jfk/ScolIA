import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  annulerFacture,
  createFactureBrouillon,
  emitFacture,
  enregistrerEncaissementFacture,
  generateFacturePdf,
  listFactures,
  listTarifs,
  noterRelanceFacture,
  solderFacture,
  upsertFoyerFacturation,
  upsertTarif,
} from "@/app/lib/facturation-db";

export async function GET(req: Request) {
  const gate = await requireModule("facturation-familles");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const view = url.searchParams.get("view") || "all";
  const [tarifs, factures] = await Promise.all([
    listTarifs(etabId),
    listFactures(etabId, {
      foyerId: url.searchParams.get("foyerId") || undefined,
    }),
  ]);
  return NextResponse.json({ tarifs, factures, view });
}

export async function POST(req: Request) {
  const gate = await requireModule("facturation-familles");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "upsertTarif");

  try {
    if (action === "upsertTarif") {
      const row = await upsertTarif(etabId, body);
      return NextResponse.json({ ok: true, tarif: row });
    }
    if (action === "createFacture") {
      const facture = await createFactureBrouillon(etabId, {
        foyerId: String(body.foyerId || ""),
        numero: String(body.numero || `FAC-${Date.now()}`),
        anneeScolaireId: body.anneeScolaireId || null,
        lignes: Array.isArray(body.lignes) ? body.lignes : [],
      });
      return NextResponse.json({ ok: true, facture });
    }
    if (action === "emitFacture") {
      const row = await emitFacture(etabId, String(body.factureId || ""));
      return NextResponse.json({ ok: true, facture: row });
    }
    if (action === "generatePdf") {
      const result = await generateFacturePdf(etabId, String(body.factureId || ""));
      return NextResponse.json({ ok: true, ...result });
    }
    if (action === "upsertFoyerFacturation") {
      const row = await upsertFoyerFacturation(etabId, body);
      return NextResponse.json({ ok: true, facturation: row });
    }
    if (action === "encaisser" || action === "enregistrerEncaissement") {
      const result = await enregistrerEncaissementFacture(etabId, {
        factureId: String(body.factureId || ""),
        montant: body.montant,
        mode: body.mode,
        dateEncaissement: body.dateEncaissement,
        reference: body.reference,
      });
      return NextResponse.json({ ok: true, ...result });
    }
    if (action === "solderFacture") {
      const result = await solderFacture(etabId, String(body.factureId || ""), {
        mode: body.mode,
        reference: body.reference,
      });
      return NextResponse.json({ ok: true, ...result });
    }
    if (action === "annulerFacture") {
      const row = await annulerFacture(etabId, String(body.factureId || ""));
      return NextResponse.json({ ok: true, facture: row });
    }
    if (action === "noterRelance") {
      const result = await noterRelanceFacture(
        etabId,
        String(body.factureId || ""),
        body.note ? String(body.note) : undefined,
      );
      return NextResponse.json({ ok: true, ...result });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur facturation." },
      { status: 400 },
    );
  }
}
