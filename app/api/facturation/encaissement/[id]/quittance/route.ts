import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getQuittanceBundle } from "@/app/lib/facturation-db";
import { renderQuittancePdfBuffer } from "@/app/lib/facturation-quittance-pdf";

type Ctx = { params: Promise<{ id: string }> };

/** Quittance PDF à la volée (l’encaissement est la quittance). */
export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("facturation-familles");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const { id } = await ctx.params;
  const bundle = await getQuittanceBundle(etabId, id);
  if (!bundle) return NextResponse.json({ error: "Encaissement introuvable." }, { status: 404 });

  const pdf = await renderQuittancePdfBuffer({
    numeroQuittance: `Q-${bundle.encaissement.id.slice(0, 8).toUpperCase()}`,
    dateEncaissement: String(bundle.encaissement.dateEncaissement),
    mode: bundle.encaissement.mode,
    montant: String(bundle.encaissement.montant),
    reference: bundle.encaissement.reference,
    foyerLabel: bundle.foyer?.label || "Foyer",
    factures: bundle.lignes.map((l) => ({
      numero: l.numero,
      montant: String(l.montant),
    })),
  });

  return new NextResponse(new Uint8Array(pdf), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `inline; filename="quittance-${id.slice(0, 8)}.pdf"`,
    },
  });
}
