import { NextResponse } from "next/server";

import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import {
  defaultStageConstraintsConfig,
  normalizeBlockedPeriod,
  normalizeCycleConstraints,
  normalizeStageConstraintsConfig,
  getStageConstraintsConfig,
  saveStageConstraintsConfig,
  type StageBlockedPeriod,
  type StageConstraintsConfig,
  type StageCycleConstraints,
} from "@/app/lib/stage-constraints-config";
import { currentStageSchoolYear, stageUid } from "@/app/lib/stage-types";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Administratif";
}

function parseBlocked(raw: unknown): StageBlockedPeriod | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  return normalizeBlockedPeriod({
    ...o,
    id: String(o.id ?? "").trim() || stageUid("blk"),
  });
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
    const config = await getStageConstraintsConfig(schoolYear);
    return NextResponse.json({
      schoolYear,
      config,
      currentSchoolYear: currentStageSchoolYear(),
      defaults: defaultStageConstraintsConfig(schoolYear),
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
    const defaults = defaultStageConstraintsConfig(schoolYear);
    const byCycleRaw =
      body.byCycle && typeof body.byCycle === "object"
        ? (body.byCycle as Record<string, unknown>)
        : {};

    const byCycle: StageConstraintsConfig["byCycle"] = {
      college: normalizeCycleConstraints(
        byCycleRaw.college,
        defaults.byCycle.college,
      ) as StageCycleConstraints,
      lycee: normalizeCycleConstraints(byCycleRaw.lycee, defaults.byCycle.lycee),
      ecole: normalizeCycleConstraints(byCycleRaw.ecole, defaults.byCycle.ecole),
    };

    const blockedPeriods: StageBlockedPeriod[] = Array.isArray(body.blockedPeriods)
      ? (body.blockedPeriods as unknown[])
          .map(parseBlocked)
          .filter((p): p is StageBlockedPeriod => p !== null)
      : [];

    const saved = await saveStageConstraintsConfig(
      normalizeStageConstraintsConfig(
        {
          schoolYear,
          updatedAt: new Date().toISOString(),
          updatedBy: displayName(user),
          byCycle,
          blockedPeriods,
        },
        schoolYear,
      ),
    );

    return NextResponse.json({ success: true, config: saved });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
