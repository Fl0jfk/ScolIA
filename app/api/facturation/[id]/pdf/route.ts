import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getFactureWithLignes } from "@/app/lib/facturation-db";
import { getObjectBytes } from "@/app/lib/s3-storage";

type Ctx = { params: Promise<{ id: string }> };

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("facturation-familles");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const { id } = await ctx.params;
  const bundle = await getFactureWithLignes(etabId, id);
  if (!bundle?.facture.pdfKey) {
    return NextResponse.json({ error: "PDF non généré." }, { status: 404 });
  }

  const bytes = await getObjectBytes(bundle.facture.pdfKey);
  if (!bytes) return NextResponse.json({ error: "Fichier PDF introuvable." }, { status: 404 });

  return new NextResponse(new Uint8Array(bytes), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `inline; filename="facture-${bundle.facture.numero}.pdf"`,
    },
  });
}
