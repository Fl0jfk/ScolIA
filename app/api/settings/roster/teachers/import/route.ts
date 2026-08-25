import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { importAssignmentsFromStageReferents, loadSchoolRoster, saveSchoolRoster } from "@/app/lib/school-roster";
import { listTeacherDirectoryOptions } from "@/app/lib/school-roster";
import { parseTeacherRosterExcelBuffer } from "@/app/lib/teacher-roster-import";

export async function POST(req: Request) {
  const gate = await requireModule("admin-settings");
  if (!gate.ok) return gate.response;
  const formData = await req.formData();
  const file = formData.get("file");
  if (!(file instanceof File)) {
    return NextResponse.json({ error: "Fichier requis." }, { status: 400 });
  }
  const name = file.name.toLowerCase();
  if (!name.endsWith(".xlsx") && !name.endsWith(".xls")) {
    return NextResponse.json({ error: "Format Excel (.xlsx) requis." }, { status: 400 });
  }
  const users = await listTeacherDirectoryOptions();
  const parsed = parseTeacherRosterExcelBuffer(await file.arrayBuffer(), users);
  if (!parsed.ok) return NextResponse.json({ error: parsed.error }, { status: 400 });

  const current = await loadSchoolRoster();
  const map = new Map(current.classAssignments.map((a) => [a.className.toLowerCase(), a]));
  for (const a of parsed.assignments) {
    map.set(a.className.toLowerCase(), a);
  }
  const roster = await saveSchoolRoster({
    teacherCatalog: current.teacherCatalog,
    classAssignments: Array.from(map.values()),
    updatedBy: gate.ctx.userId,
  });

  return NextResponse.json({
    ok: true,
    imported: parsed.assignments.length,
    roster,
    message: `${parsed.assignments.length} affectation(s) importée(s). Synchronisé avec Stages et répartition des classes.`,
  });
}
