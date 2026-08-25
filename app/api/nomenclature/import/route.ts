import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { importSiecleXmlBuffersBatch } from "@/app/lib/nomenclature-import/siecle-xml";
import { buildNomenclatureImportAnomalies } from "@/app/lib/nomenclature-import/import-anomalies";
import { getDb } from "@/db/index";
import { nomenclatureImportLog, refEtablissement, refNomenclature } from "@/db/schema";
import { desc, eq, sql } from "drizzle-orm";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const db = getDb();
  const [counts, logs, refEtabCount, anomalies] = await Promise.all([
    db
      .select({
        type: refNomenclature.type,
        n: sql<number>`count(*)::int`,
      })
      .from(refNomenclature)
      .where(eq(refNomenclature.etablissementId, etabId))
      .groupBy(refNomenclature.type),
    db
      .select({
        id: nomenclatureImportLog.id,
        fichier: nomenclatureImportLog.fichier,
        statut: nomenclatureImportLog.statut,
        nbInserts: nomenclatureImportLog.nbInserts,
        nbUpdates: nomenclatureImportLog.nbUpdates,
        dateImport: nomenclatureImportLog.dateImport,
        rapportJson: nomenclatureImportLog.rapportJson,
      })
      .from(nomenclatureImportLog)
      .where(eq(nomenclatureImportLog.etablissementId, etabId))
      .orderBy(desc(nomenclatureImportLog.dateImport))
      .limit(20),
    db
      .select({ n: sql<number>`count(*)::int` })
      .from(refEtablissement)
      .then((rows) => rows[0]?.n ?? 0),
    buildNomenclatureImportAnomalies(etabId),
  ]);

  return NextResponse.json({ counts, logs, refEtablissementCount: refEtabCount, anomalies });
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const form = await req.formData().catch(() => null);
  if (!form) return NextResponse.json({ error: "Formulaire invalide." }, { status: 400 });

  const files = form.getAll("files").filter((f): f is File => f instanceof File);
  const single = form.get("file");
  if (single instanceof File) files.push(single);
  if (!files.length) {
    return NextResponse.json({ error: "Aucun fichier XML." }, { status: 400 });
  }

  const reports: Array<{ file: string; error?: string; kind?: string; message?: string }> = [];
  const fileBuffers: Array<{ filename: string; buffer: ArrayBuffer }> = [];

  for (const file of files) {
    if (!/\.xml$/i.test(file.name)) {
      reports.push({ file: file.name, error: "Extension .xml attendue." });
      continue;
    }
    const buf = await file.arrayBuffer();
    fileBuffers.push({ filename: file.name, buffer: buf });
  }

  const batchReports = await importSiecleXmlBuffersBatch(etabId, fileBuffers);
  for (const r of batchReports) {
    reports.push(r);
  }

  return NextResponse.json({ ok: true, reports });
}
