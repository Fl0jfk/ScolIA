import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  loadOfficialSchoolClasses,
  listUnmatchedEleveClasses,
  RECTORAT_LOCKED_POLES,
} from "@/app/lib/nomenclature-classes";
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

  const lockedDivisions = official.divisions.filter(
    (d) =>
      official.lockedClassesByPole.COLLÈGE?.includes(d.code) ||
      official.lockedClassesByPole.LYCÉE?.includes(d.code),
  );

  return NextResponse.json({
    hasLockedSiecle: official.hasLockedSiecle,
    lockedPoles: [...RECTORAT_LOCKED_POLES],
    lockedClasses: official.lockedClasses,
    lockedClassesByPole: official.lockedClassesByPole,
    classesByPole: official.classesByPole,
    divisions: lockedDivisions.map((d) => ({
      code: d.code,
      libelle: d.libelleLong || d.libelleCourt || d.code,
      metadata: d.metadataJson,
    })),
    ecoleFromSiecle: official.classesByPole.ÉCOLE || [],
    unmatchedEleveClasses: unmatched,
    readOnly: false,
    siecleLockedCollègeLycée: official.hasLockedSiecle,
  });
}
