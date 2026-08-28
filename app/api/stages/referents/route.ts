import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { listStageEnabledClassNames } from "@/app/lib/stage-periods-config";
import {
  getStageReferentsConfig,
  saveStageReferentsConfig,
  type StageClassReferentAssignment,
  type StageReferentsConfig,
} from "@/app/lib/stage-referents-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Administratif";
}

function parseAssignments(raw: unknown): StageClassReferentAssignment[] {
  if (!Array.isArray(raw)) return [];
  const out: StageClassReferentAssignment[] = [];
  for (const row of raw) {
    if (!row || typeof row !== "object") continue;
    const o = row as Record<string, unknown>;
    const className = String(o.className ?? "").trim();
    const externalUserId = String(o.externalUserId ?? "").trim();
    const name = String(o.name ?? "").trim();
    const email = String(o.email ?? "").trim().toLowerCase();
    if (!className || !externalUserId || !name || !email) continue;
    out.push({ className, externalUserId, name, email });
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
    const config = await getStageReferentsConfig(schoolYear);
    const classes = await listStageEnabledClassNames(schoolYear);

    const prevYearParts = schoolYear.split("-").map(Number);
    const prevYear =
      prevYearParts.length === 2 && prevYearParts.every((n) => !Number.isNaN(n))
        ? `${prevYearParts[0]! - 1}-${prevYearParts[1]! - 1}`
        : null;
    const previousConfig = prevYear ? await getStageReferentsConfig(prevYear) : null;

    return NextResponse.json({
      schoolYear,
      config,
      classes,
      previousConfig,
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

    const config: StageReferentsConfig = {
      schoolYear,
      updatedAt: new Date().toISOString(),
      updatedBy: displayName(user),
      assignments,
    };
    const saved = await saveStageReferentsConfig(config);
    return NextResponse.json({ success: true, config: saved });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
