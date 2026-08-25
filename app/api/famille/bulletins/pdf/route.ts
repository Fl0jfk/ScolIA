import { NextResponse } from "next/server";
import { requireFamilleEleveAccess } from "@/app/lib/famille-auth";
import { loadBulletinSnapshot } from "@/app/lib/notes-bulletins-db";
import { bulletinPdfFilename, renderBulletinPdfBuffer } from "@/app/lib/notes-bulletin-pdf";

export async function GET(req: Request) {
  const url = new URL(req.url);
  const eleveId = url.searchParams.get("eleveId")?.trim() || "";
  const periodeId = url.searchParams.get("periodeId")?.trim() || "";

  if (!eleveId || !periodeId) {
    return NextResponse.json({ error: "eleveId et periodeId requis." }, { status: 400 });
  }

  const gate = await requireFamilleEleveAccess(eleveId);
  if (!gate.ok) return gate.response;

  const snapshot = await loadBulletinSnapshot(gate.ctx.etablissementId, eleveId, periodeId);
  if (!snapshot) {
    return NextResponse.json({ error: "Bulletin introuvable." }, { status: 404 });
  }

  if (snapshot.periode.statut !== "cloturee") {
    return NextResponse.json(
      { error: "Ce bulletin n'est pas encore publié.", code: "BULLETIN_NOT_PUBLISHED" },
      { status: 403 },
    );
  }

  if (!snapshot.lignes.length && !snapshot.competences.length) {
    return NextResponse.json({ error: "Bulletin vide." }, { status: 404 });
  }

  const buffer = await renderBulletinPdfBuffer(snapshot);
  return new NextResponse(new Uint8Array(buffer), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `inline; filename="${bulletinPdfFilename(snapshot)}"`,
      "Cache-Control": "private, no-store",
    },
  });
}
