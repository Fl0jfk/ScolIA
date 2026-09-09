import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import {
  getStageWatchersConfig,
  listStageWatcherClassNames,
  saveStageWatchersConfig,
  type StageWatcherAssignment,
  type StageWatcherKind,
  type StageWatchersConfig,
} from "@/app/lib/stage-watchers-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Administratif";
}

function parseAssignments(raw: unknown): StageWatcherAssignment[] {
  if (!Array.isArray(raw)) return [];
  const out: StageWatcherAssignment[] = [];
  for (const row of raw) {
    if (!row || typeof row !== "object") continue;
    const o = row as Record<string, unknown>;
    const externalUserId = String(o.externalUserId ?? "").trim();
    const name = String(o.name ?? "").trim();
    const email = String(o.email ?? "").trim().toLowerCase();
    const kind: StageWatcherKind = o.kind === "restauration" ? "restauration" : "cpe";
    if (!externalUserId || !name || !email) continue;
    if (o.scope === "student") {
      const studentFirstName = String(o.studentFirstName ?? "").trim();
      const studentLastName = String(o.studentLastName ?? "").trim();
      const studentClassName = String(o.studentClassName ?? "").trim();
      if (!studentFirstName || !studentLastName) continue;
      out.push({
        scope: "student",
        studentKey: String(o.studentKey ?? "").trim() ||
          `${studentLastName.toLowerCase()}|${studentFirstName.toLowerCase()}|${studentClassName.toLowerCase()}`,
        studentFirstName,
        studentLastName,
        studentClassName,
        externalUserId,
        name,
        email,
        kind,
      });
    } else {
      const className = String(o.className ?? "").trim();
      if (!className) continue;
      out.push({ scope: "class", className, externalUserId, name, email, kind });
    }
  }
  return out;
}

export async function GET(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles)) {
      return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
    }

    const { searchParams } = new URL(req.url);
    const schoolYear = searchParams.get("schoolYear")?.trim() || currentStageSchoolYear();
    const config = await getStageWatchersConfig(schoolYear);
    const classes = await listStageWatcherClassNames(schoolYear);

    return NextResponse.json({
      schoolYear,
      config,
      classes,
      currentSchoolYear: currentStageSchoolYear(),
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles)) {
      return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
    }

    const body = await req.json();
    const schoolYear = String(body.schoolYear ?? "").trim() || currentStageSchoolYear();
    const assignments = parseAssignments(body.assignments);
    const next: StageWatchersConfig = {
      schoolYear,
      updatedAt: new Date().toISOString(),
      updatedBy: displayName(user),
      assignments,
    };
    const saved = await saveStageWatchersConfig(next);
    return NextResponse.json({ success: true, config: saved });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
