import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { classifyRegime } from "@/app/lib/eleve-regime";
import { countElevesRegistry, loadElevesRegistry } from "@/app/lib/eleves-registry";
import {
  loadSchoolRoster,
  listTeacherDirectoryOptions,
  saveSchoolRoster,
  type SchoolRosterConfig,
} from "@/app/lib/school-roster";
import { listStageReferentClassNames } from "@/app/lib/stage-referents-config";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const [roster, elevesCount, users, classesFromStages, eleves] = await Promise.all([
    loadSchoolRoster(),
    countElevesRegistry(),
    listTeacherDirectoryOptions(),
    listStageReferentClassNames(),
    loadElevesRegistry(),
  ]);

  const elevesSansClasse = eleves.filter((e) => !String(e.classe || "").trim()).length;
  const elevesSansIne = eleves.filter((e) => !String(e.ine || "").trim()).length;
  const regimeCounts = { interne: 0, demi_pension: 0, externe: 0, inconnu: 0 };
  for (const e of eleves) {
    regimeCounts[classifyRegime(e.regime)] += 1;
  }
  const classesFromEleves = [
    ...new Set(
      eleves
        .map((e) => String(e.classe || "").trim())
        .filter(Boolean),
    ),
  ].sort((a, b) => a.localeCompare(b, "fr"));
  const classes = [
    ...new Set([...classesFromStages, ...classesFromEleves, ...roster.classAssignments.map((a) => a.className)]),
  ].sort((a, b) => a.localeCompare(b, "fr"));

  return NextResponse.json({
    roster,
    elevesCount,
    elevesSansClasse,
    elevesSansIne,
    regimeCounts,
    users,
    classes,
  });
}

export async function PUT(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const body = (await req.json()) as Partial<SchoolRosterConfig>;
  const current = await loadSchoolRoster();
  const roster = await saveSchoolRoster({
    teacherCatalog: Array.isArray(body.teacherCatalog) ? body.teacherCatalog : current.teacherCatalog,
    classAssignments: Array.isArray(body.classAssignments)
      ? body.classAssignments
      : current.classAssignments,
    updatedBy: gate.ctx.userId,
  });
  return NextResponse.json({ ok: true, roster });
}
