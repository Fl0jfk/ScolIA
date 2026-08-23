import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { loadCampaignConfig } from "@/app/lib/class-allocation-storage";
import {
  importAssignmentsFromStageReferents,
  loadSchoolRoster,
  listTeacherDirectoryOptions,
  saveSchoolRoster,
} from "@/app/lib/school-roster";
import { listStageReferentClassNames } from "@/app/lib/stage-referents-config";

/** Compatibilité : délègue au référentiel global des paramètres. */
export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const campaign = await loadCampaignConfig();
  const [roster, users, classes] = await Promise.all([
    loadSchoolRoster(),
    listTeacherDirectoryOptions(),
    listStageReferentClassNames(),
  ]);
  return NextResponse.json({
    campaignId: campaign.id,
    config: {
      campaignId: campaign.id,
      assignments: roster.classAssignments,
      updatedAt: roster.updatedAt,
    },
    classes,
    users,
  });
}

export async function PUT(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const body = await req.json();
  const action = String(body.action || "save");
  const current = await loadSchoolRoster();
  if (action === "import_from_stages") {
    const imported = await importAssignmentsFromStageReferents(gate.ctx.userId);
    return NextResponse.json({ ok: true, config: { assignments: imported.classAssignments } });
  }
  const roster = await saveSchoolRoster({
    teacherCatalog: current.teacherCatalog,
    classAssignments: Array.isArray(body.assignments) ? body.assignments : current.classAssignments,
    updatedBy: gate.ctx.userId,
  });
  return NextResponse.json({ ok: true, config: { assignments: roster.classAssignments } });
}
