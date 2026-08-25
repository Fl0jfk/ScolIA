import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  buildSiecleExportBundle,
  recordSiecleExportLog,
} from "@/app/lib/nomenclature-import/export-siecle";
import { getDb } from "@/db/index";
import { desc, eq } from "drizzle-orm";
import { nomenclatureImportLog } from "@/db/schema";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const db = getDb();
  const logs = await db
    .select()
    .from(nomenclatureImportLog)
    .where(eq(nomenclatureImportLog.etablissementId, etabId))
    .orderBy(desc(nomenclatureImportLog.dateImport))
    .limit(30);

  const exports = logs
    .filter((l) => {
      const rapport = l.rapportJson as { sens?: string } | null;
      return rapport?.sens === "export" || l.source === "siecle_export";
    })
    .map((l) => ({
      id: l.id,
      fichier: l.fichier,
      dateImport: l.dateImport,
      statut: l.statut,
      rapport: l.rapportJson,
    }));

  return NextResponse.json({ exports });
}

export async function POST() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  try {
    const bundle = await buildSiecleExportBundle(etabId);
    await recordSiecleExportLog(etabId, bundle);

    return new NextResponse(new Uint8Array(bundle.zipBuffer), {
      headers: {
        "Content-Type": "application/zip",
        "Content-Disposition": `attachment; filename="${bundle.filename}"`,
        "X-Scolia-Export-Stats": JSON.stringify(bundle.stats),
      },
    });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Export Siècle impossible." },
      { status: 400 },
    );
  }
}
