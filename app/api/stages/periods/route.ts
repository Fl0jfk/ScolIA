import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { loadAppConfig } from "@/app/lib/app-config";
import { sanitizeDomainPlanningClassesByPole } from "@/app/lib/domain-planning-defaults";
import {
  getStagePeriodsConfig,
  saveStagePeriodsConfig,
  type StageClassStageConfig,
  type StagePeriodReminder,
  type StageClassPeriod,
} from "@/app/lib/stage-periods-config";
import { currentStageSchoolYear, stageUid } from "@/app/lib/stage-types";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Administratif";
}

function parseReminder(raw: unknown): StagePeriodReminder | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = String(o.id ?? "").trim() || stageUid("rem");
  const label = String(o.label ?? "").trim();
  const message = String(o.message ?? "").trim();
  if (!label || !message) return null;
  return {
    id,
    label,
    message,
    periodStart: typeof o.periodStart === "string" ? o.periodStart.slice(0, 10) : undefined,
    periodEnd: typeof o.periodEnd === "string" ? o.periodEnd.slice(0, 10) : undefined,
  };
}

function parsePeriod(raw: unknown): StageClassPeriod | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = String(o.id ?? "").trim() || stageUid("per");
  const label = String(o.label ?? "").trim();
  const periodStart = typeof o.periodStart === "string" ? o.periodStart.slice(0, 10) : "";
  const periodEnd = typeof o.periodEnd === "string" ? o.periodEnd.slice(0, 10) : "";
  if (!label || !periodStart || !periodEnd) return null;
  return { id, label, periodStart, periodEnd };
}

function parseClassConfig(raw: unknown): StageClassStageConfig | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const className = String(o.className ?? "").trim();
  if (!className) return null;
  const enabled = o.enabled !== false;
  const periods = Array.isArray(o.periods)
    ? o.periods.map(parsePeriod).filter((p): p is StageClassPeriod => p !== null)
    : [];
  const reminders = Array.isArray(o.reminders)
    ? o.reminders.map(parseReminder).filter((r): r is StagePeriodReminder => r !== null)
    : [];
  return { className, enabled, periods, reminders };
}

async function listAllPlanningClasses(): Promise<string[]> {
  const bundle = await loadAppConfig();
  const poles = sanitizeDomainPlanningClassesByPole(bundle.domainPlanning.classesByPole || {});
  const set = new Set<string>();
  for (const c of Object.values(poles).flat()) {
    const n = String(c ?? "").trim();
    if (n) set.add(n);
  }
  return [...set].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
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
    const config = await getStagePeriodsConfig(schoolYear);
    const planningClasses = await listAllPlanningClasses();

    const prevYearParts = schoolYear.split("-").map(Number);
    const prevYear =
      prevYearParts.length === 2 && prevYearParts.every((n) => !Number.isNaN(n))
        ? `${prevYearParts[0]! - 1}-${prevYearParts[1]! - 1}`
        : null;
    const previousConfig = prevYear ? await getStagePeriodsConfig(prevYear) : null;

    return NextResponse.json({
      schoolYear,
      config,
      suggestedClasses: planningClasses,
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
    const classes: StageClassStageConfig[] = Array.isArray(body.classes)
      ? (body.classes as unknown[])
          .map(parseClassConfig)
          .filter((c): c is StageClassStageConfig => c !== null)
      : [];

    const saved = await saveStagePeriodsConfig({
      schoolYear,
      updatedAt: new Date().toISOString(),
      updatedBy: displayName(user),
      classes,
    });
    return NextResponse.json({ success: true, config: saved });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
