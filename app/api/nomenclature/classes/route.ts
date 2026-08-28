import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { loadOfficialSchoolClasses, listUnmatchedEleveClasses } from "@/app/lib/nomenclature-classes";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const official = await loadOfficialSchoolClasses(etabId);
  const eleves = await loadElevesRegistry();
  const eleveClasses = eleves.map((e) => String(e.classe || "").trim()).filter(Boolean);
  const unmatched = await listUnmatchedEleveClasses(etabId, eleveClasses);

  return NextResponse.json({
    source: official.source,
    classes: official.classes,
    classesByPole: official.classesByPole,
    divisions: official.divisions.map((d) => ({
      code: d.code,
      libelle: d.libelleLong || d.libelleCourt || d.code,
      metadata: d.metadataJson,
    })),
    unmatchedEleveClasses: unmatched,
    readOnly: official.source === "siecle",
  });
}
