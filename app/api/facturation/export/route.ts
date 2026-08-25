import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { listFacturationExportRows } from "@/app/lib/facturation-db";
import { buildFacturationExportCsv } from "@/app/lib/facturation-export";
import { parisDateKey } from "@/app/lib/paris-time";

export async function GET(req: Request) {
  const gate = await requireModule("facturation-familles");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const statut = url.searchParams.get("statut") || undefined;
  const todayIso = parisDateKey(new Date());
  const rows = await listFacturationExportRows(etabId, { statut, todayIso });
  const csv = buildFacturationExportCsv(rows);
  const stamp = todayIso.replace(/-/g, "");
  const suffix = statut ? `-${statut}` : "";

  return new NextResponse(csv, {
    headers: {
      "Content-Type": "text/csv; charset=utf-8",
      "Content-Disposition": `attachment; filename="facturation-export${suffix}-${stamp}.csv"`,
      "Cache-Control": "no-store",
    },
  });
}
