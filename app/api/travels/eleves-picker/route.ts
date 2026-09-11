import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { isEleveScolarise } from "@/app/lib/eleves-config";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import {
  filterObservedClassesForCurrentYearUi,
  loadOfficialSchoolClasses,
} from "@/app/lib/nomenclature-classes";
import { writeDataAccessAudit } from "@/app/lib/data-access-audit";

/**
 * Élèves scolarisés + classes année en cours pour le sélecteur Voyages.
 * Exclut les libellés N-1 / hors Structures Siècle (ex. 03D, 3ème B).
 */
export async function GET(req: Request) {
  const gate = await requireModule("travels");
  if (!gate.ok) return gate.response;

  try {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) {
      return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
    }

    const [elevesRaw, official] = await Promise.all([
      loadElevesActifsRegistry(),
      loadOfficialSchoolClasses(etabId),
    ]);

    const eleves = elevesRaw.filter(isEleveScolarise);
    const observedClasses = eleves
      .map((e) => String(e.classe || "").trim())
      .filter(Boolean);
    const classes = filterObservedClassesForCurrentYearUi(observedClasses, official);

    await writeDataAccessAudit({
      etablissementId: etabId,
      userId: gate.ctx.user.id,
      resourceType: "eleves_registry",
      action: "list",
      req,
      metadata: { count: eleves.length, source: "travels-eleves-picker" },
    });

    return NextResponse.json({
      eleves,
      classes,
      hasLockedSiecle: official.hasLockedSiecle,
      count: eleves.length,
    });
  } catch (err: unknown) {
    console.error("[travels/eleves-picker]", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Chargement élèves impossible." },
      { status: 500 },
    );
  }
}
