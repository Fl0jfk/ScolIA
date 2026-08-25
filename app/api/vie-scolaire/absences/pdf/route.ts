import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getTenant } from "@/app/lib/tenant-context";
import { listAbsencesATraiter } from "@/app/lib/vs-absences-db";
import { renderAbsencesCpePdfBuffer } from "@/app/lib/vs-absences-pdf";

export async function GET(req: Request) {
  const gate = await requireModule("vs-absences");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const statut = (url.searchParams.get("statut")?.trim() || "a_traiter") as
    | "a_traiter"
    | "justifiee"
    | "non_justifiee"
    | "classee";

  const rows = await listAbsencesATraiter(etabId, { statut, limit: 500 });
  let etabLabel = "Établissement";
  try {
    const tenant = await getTenant();
    etabLabel = tenant.label || tenant.slug || etabLabel;
  } catch {
    /* ignore */
  }

  const buffer = renderAbsencesCpePdfBuffer({
    etablissementLabel: etabLabel,
    rows: rows.map((r) => ({
      eleveNom: r.eleveNom,
      elevePrenom: r.elevePrenom,
      eleveClasse: r.eleveClasse,
      dateDebut: r.dateDebut,
      type: r.type,
      statut: r.statut,
      justifie: r.justifie,
      motif: r.motif,
    })),
  });

  const filename = `absences-${statut}-${new Date().toISOString().slice(0, 10)}.pdf`;
  return new NextResponse(new Uint8Array(buffer), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `attachment; filename="${filename}"`,
      "Cache-Control": "private, no-store",
    },
  });
}
