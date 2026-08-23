import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { countElevesRegistry } from "@/app/lib/eleves-registry";
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
  const [roster, elevesCount, users, classes] = await Promise.all([
    loadSchoolRoster(),
    countElevesRegistry(),
    listTeacherDirectoryOptions(),
    listStageReferentClassNames(),
  ]);
  return NextResponse.json({ roster, elevesCount, users, classes });
}

export async function PUT(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const body = (await req.json()) as Partial<SchoolRosterConfig>;
  const current = await loadSchoolRoster();
  const roster = await saveSchoolRoster({
    teacherCatalog: Array.isArray(body.teacherCatalog) ? body.teacherCatalog : current.teacherCatalog,
    classAssignments: Array.isArray(body.classAssignments) ? body.classAssignments : current.classAssignments,
    updatedBy: gate.ctx.userId,
  });
  return NextResponse.json({ ok: true, roster });
}
