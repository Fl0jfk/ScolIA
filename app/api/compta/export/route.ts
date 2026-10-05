import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  exportCabinetToCsv,
  listExportCabinet,
} from "@/app/lib/compta-etablissement-db";

/** Fichier pour l’expert-comptable (vue + dépenses) — pas le module. */
export async function GET(req: Request) {
  const gate = await requireModule("compta-etablissement");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const from = url.searchParams.get("from") || undefined;
  const to = url.searchParams.get("to") || undefined;
  const rows = await listExportCabinet(etabId, { from, to });
  const csv = exportCabinetToCsv(rows);
  const stamp = new Date().toISOString().slice(0, 10);

  return new NextResponse(csv, {
    headers: {
      "Content-Type": "text/csv; charset=utf-8",
      "Content-Disposition": `attachment; filename="export-cabinet-${stamp}.csv"`,
    },
  });
}
