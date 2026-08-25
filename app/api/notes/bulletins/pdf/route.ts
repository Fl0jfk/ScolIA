import { NextResponse } from "next/server";
import JSZip from "jszip";
import { requireAdmin, requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  listEleveIdsForBulletinClasse,
  listEleveIdsForBulletinGroupe,
  loadBulletinSnapshot,
} from "@/app/lib/notes-bulletins-db";
import { bulletinPdfFilename, renderBulletinPdfBuffer } from "@/app/lib/notes-bulletin-pdf";

export async function GET(req: Request) {
  const gate = await requireModule("notes");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const eleveId = url.searchParams.get("eleveId")?.trim() || "";
  const periodeId = url.searchParams.get("periodeId")?.trim() || "";
  const classe = url.searchParams.get("classe")?.trim() || "";
  const groupeId = url.searchParams.get("groupeId")?.trim() || "";
  const mode = url.searchParams.get("mode") || "single";

  if (!periodeId) {
    return NextResponse.json({ error: "Période requise." }, { status: 400 });
  }

  if (mode === "classe") {
    const adminGate = await requireAdmin();
    if (!adminGate.ok) return adminGate.response;
    if (!classe && !groupeId) {
      return NextResponse.json({ error: "Classe ou groupe requis pour l'export ZIP." }, { status: 400 });
    }

    const eleves = groupeId
      ? await listEleveIdsForBulletinGroupe(etabId, groupeId)
      : await listEleveIdsForBulletinClasse(etabId, classe);
    if (!eleves.length) {
      return NextResponse.json({ error: "Aucun élève dans ce périmètre." }, { status: 404 });
    }

    const zip = new JSZip();
    let added = 0;
    for (const e of eleves) {
      const snapshot = await loadBulletinSnapshot(etabId, e.id, periodeId);
      if (!snapshot || (snapshot.lignes.length === 0 && snapshot.competences.length === 0)) continue;
      const buffer = await renderBulletinPdfBuffer(snapshot);
      zip.file(bulletinPdfFilename(snapshot), buffer);
      added += 1;
    }

    if (added === 0) {
      return NextResponse.json(
        { error: "Aucune moyenne enregistrée pour ce périmètre et cette période." },
        { status: 404 },
      );
    }

    const zipBuffer = await zip.generateAsync({ type: "nodebuffer", compression: "DEFLATE" });
    const safeLabel = (classe || groupeId.slice(0, 8)).replace(/[^a-zA-Z0-9_-]+/g, "-");
    return new NextResponse(new Uint8Array(zipBuffer), {
      headers: {
        "Content-Type": "application/zip",
        "Content-Disposition": `attachment; filename="bulletins-${safeLabel}.zip"`,
      },
    });
  }

  if (!eleveId) {
    return NextResponse.json({ error: "Élève requis." }, { status: 400 });
  }

  const snapshot = await loadBulletinSnapshot(etabId, eleveId, periodeId);
  if (!snapshot) {
    return NextResponse.json({ error: "Bulletin introuvable." }, { status: 404 });
  }

  const buffer = await renderBulletinPdfBuffer(snapshot);
  return new NextResponse(new Uint8Array(buffer), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `inline; filename="${bulletinPdfFilename(snapshot)}"`,
    },
  });
}
