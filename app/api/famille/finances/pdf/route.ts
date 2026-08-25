import { NextResponse } from "next/server";
import { and, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleveFoyerLink, facture } from "@/db/schema";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { getFactureWithLignes } from "@/app/lib/facturation-db";
import { getObjectBytes } from "@/app/lib/s3-storage";

/** PDF facture — uniquement si la facture appartient à un foyer d’un enfant du parent. */
export async function GET(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const factureId = new URL(req.url).searchParams.get("factureId")?.trim() || "";
  if (!factureId) {
    return NextResponse.json({ error: "factureId requis." }, { status: 400 });
  }

  const etabId = gate.ctx.etablissementId;
  const eleveIds = gate.ctx.enfants.map((e) => e.id);
  const db = getDb();

  const foyerLinks = await db
    .select({ foyerId: eleveFoyerLink.foyerId })
    .from(eleveFoyerLink)
    .where(
      and(
        eq(eleveFoyerLink.etablissementId, etabId),
        inArray(eleveFoyerLink.eleveId, eleveIds),
      ),
    );
  const foyerIds = [...new Set(foyerLinks.map((l) => l.foyerId))];
  if (!foyerIds.length) {
    return NextResponse.json({ error: "Aucune facture accessible." }, { status: 403 });
  }

  const [row] = await db
    .select({ id: facture.id, foyerId: facture.foyerId, statut: facture.statut })
    .from(facture)
    .where(and(eq(facture.etablissementId, etabId), eq(facture.id, factureId)))
    .limit(1);

  if (!row || !foyerIds.includes(row.foyerId) || row.statut === "brouillon") {
    return NextResponse.json({ error: "Facture introuvable." }, { status: 404 });
  }

  const bundle = await getFactureWithLignes(etabId, factureId);
  if (!bundle?.facture.pdfKey) {
    return NextResponse.json({ error: "PDF non encore disponible." }, { status: 404 });
  }

  const bytes = await getObjectBytes(bundle.facture.pdfKey);
  if (!bytes) {
    return NextResponse.json({ error: "Fichier PDF introuvable." }, { status: 404 });
  }

  return new NextResponse(new Uint8Array(bytes), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `inline; filename="facture-${bundle.facture.numero}.pdf"`,
    },
  });
}
