import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { importSiecleXmlBuffersBatch } from "@/app/lib/nomenclature-import/siecle-xml";
import { buildNomenclatureImportAnomalies } from "@/app/lib/nomenclature-import/import-anomalies";
import { buildSiecleImportStatus, SIECLE_IMPORT_SLOTS } from "@/app/lib/nomenclature-import/import-status";
import {
  parseSiecleImportCycle,
  SIECLE_IMPORT_CYCLES,
} from "@/app/lib/nomenclature-import/siecle-import-cycle";
import { getDb } from "@/db/index";
import { nomenclatureImportLog, refEtablissement, refNomenclature } from "@/db/schema";
import { desc, eq, sql } from "drizzle-orm";

/** XML Siècle Élèves/Responsables dépassent souvent 7–10 Mo. */
export const maxDuration = 300;

/** Limite métier par fichier (alignée sur proxyClientMaxBodySize 110 Mo). */
const MAX_XML_BYTES = 100 * 1024 * 1024;
const MAX_XML_LABEL = "100 Mo";

function formatMo(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
}

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const db = getDb();
  const [counts, logs, refEtabCount, anomalies, importStatus] = await Promise.all([
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
      .limit(30),
    db
      .select({ n: sql<number>`count(*)::int` })
      .from(refEtablissement)
      .then((rows) => rows[0]?.n ?? 0),
    buildNomenclatureImportAnomalies(etabId),
    buildSiecleImportStatus(etabId),
  ]);

  return NextResponse.json({
    counts,
    logs,
    refEtablissementCount: refEtabCount,
    anomalies,
    importStatus,
    slots: SIECLE_IMPORT_SLOTS,
    cycles: SIECLE_IMPORT_CYCLES,
    maxXmlBytes: MAX_XML_BYTES,
    maxXmlLabel: MAX_XML_LABEL,
  });
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const contentLength = Number(req.headers.get("content-length") || 0);
  let form: FormData | null = null;
  try {
    form = await req.formData();
  } catch (err) {
    console.error("[nomenclature/import] formData failed", {
      contentLength,
      err: err instanceof Error ? err.message : err,
    });
    const tooLargeHint =
      contentLength > 10 * 1024 * 1024
        ? ` Le corps de la requête fait ${formatMo(contentLength)} — importez les XML un par un si besoin (max ${MAX_XML_LABEL}/fichier).`
        : "";
    return NextResponse.json(
      {
        error:
          "Formulaire invalide ou fichier trop volumineux pour le serveur." + tooLargeHint,
      },
      { status: 400 },
    );
  }
  if (!form) {
    return NextResponse.json({ error: "Formulaire invalide." }, { status: 400 });
  }

  const cycle = parseSiecleImportCycle(form.get("cycle"));
  if (!cycle) {
    return NextResponse.json(
      {
        error:
          "Indiquez le cycle d'import : collège ou lycée (Siècle exporte les deux séparément).",
      },
      { status: 400 },
    );
  }

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
    if (file.size > MAX_XML_BYTES) {
      reports.push({
        file: file.name,
        error: `Fichier trop volumineux (${formatMo(file.size)}, max ${MAX_XML_LABEL}).`,
      });
      continue;
    }
    const buf = await file.arrayBuffer();
    if (!buf.byteLength) {
      reports.push({ file: file.name, error: "Fichier vide." });
      continue;
    }
    fileBuffers.push({ filename: file.name, buffer: buf });
  }

  if (!fileBuffers.length) {
    return NextResponse.json(
      {
        error:
          reports.find((r) => r.error)?.error ||
          "Aucun fichier XML valide à importer.",
        reports,
      },
      { status: 400 },
    );
  }

  const batchReports = await importSiecleXmlBuffersBatch(etabId, fileBuffers, { cycle });
  for (const r of batchReports) {
    reports.push(r);
  }

  return NextResponse.json({ ok: true, cycle, reports });
}
