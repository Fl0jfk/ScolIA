import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  listClassesForTeacherUser,
  studentInAssignedClasses,
} from "@/app/lib/class-allocation-teachers";
import { loadSchoolRoster } from "@/app/lib/school-roster";
import { requireAuth } from "@/app/lib/intranet-auth";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  canManagePersonnel,
  canViewPersonnelDashboard,
} from "@/app/lib/personnel-types";
import {
  resolveEleveLiveCourse,
} from "@/app/lib/rh/planning-class-live";
import {
  buildClassWeekSchedule,
  listClassesFromTeacherIndex,
} from "@/app/lib/rh/planning-class-schedule";
import {
  addDays,
  schoolWeekParity,
  startOfWeekMonday,
  toIsoDateLocal,
  weekDayContexts,
} from "@/app/lib/rh/planning-calendar";
import { getTeacherPlanningEntries } from "@/app/lib/rh/planning-teacher-index";

function isTeacherRole(roles: string[]) {
  return hasRole(roles, "professeur");
}

function parseWeekStart(raw: string | null): Date {
  if (raw && /^\d{4}-\d{2}-\d{2}$/.test(raw)) {
    const d = new Date(`${raw}T12:00:00`);
    if (!Number.isNaN(d.getTime())) return startOfWeekMonday(d);
  }
  return startOfWeekMonday(new Date());
}

function mergeClassOptions(
  fromEdt: string[],
  fromCatalog: string[],
  assignedOnly?: string[],
): string[] {
  const base = assignedOnly?.length
    ? assignedOnly
    : [...new Set([...fromEdt, ...fromCatalog])];
  return base.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const roles = rolesFromUserLike(user);
  const canManage = canManagePersonnel(roles);
  const canViewAll = canViewPersonnelDashboard(roles);
  const isTeacher = isTeacherRole(roles);

  if (!canManage && !canViewAll && !isTeacher) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  const url = new URL(req.url);
  const classeParam = url.searchParams.get("classe")?.trim() || "";
  const weekType = url.searchParams.get("week") === "B" ? "B" : "A";
  const weekStart = parseWeekStart(url.searchParams.get("weekStart"));

  const entries = await getTeacherPlanningEntries();
  const cfg = await loadAppConfig();
  const zone = cfg.identity.schoolHolidayZone ?? null;

  const roster = await loadSchoolRoster();
  const classesFromRoster = [
    ...new Set(roster.classAssignments.map((a) => a.className.trim()).filter(Boolean)),
  ];
  const classesFromProfRoom = Object.values(cfg.profRoom.classesByPole || {}).flat();
  const classesFromEdt = listClassesFromTeacherIndex(entries);

  let assignedClasses: string[] | null = null;
  if (isTeacher && !canManage && !canViewAll) {
    assignedClasses = await listClassesForTeacherUser(gate.ctx.userId);
  }

  const classOptions = mergeClassOptions(
    classesFromEdt,
    [...classesFromRoster, ...classesFromProfRoom],
    assignedClasses ?? undefined,
  );

  if (!classeParam) {
    return NextResponse.json({
      classes: classOptions,
      weekType,
      weekStart: toIsoDateLocal(weekStart),
      schoolHolidayZone: zone,
      profScoped: Boolean(assignedClasses),
    });
  }

  if (
    assignedClasses &&
    !assignedClasses.some((c) => studentInAssignedClasses(classeParam, [c]))
  ) {
    return NextResponse.json({ error: "Classe introuvable." }, { status: 404 });
  }

  const slots = buildClassWeekSchedule({
    classe: classeParam,
    entries,
    weekType,
    weekStart,
  });

  const live = await resolveEleveLiveCourse({
    classe: classeParam,
    zone,
    teacherEntries: entries,
  });

  const dayContexts = weekDayContexts({
    weekStart,
    audience: "teacher",
    zone,
    leaves: [],
  });

  const weekEnd = toIsoDateLocal(addDays(weekStart, 4));
  const replacementsThisWeek = slots.filter((s) => s.kind === "remplacement");

  return NextResponse.json({
    classe: classeParam,
    classes: classOptions,
    weekType,
    weekStart: toIsoDateLocal(weekStart),
    weekEnd,
    parityLabel: schoolWeekParity(new Date(`${toIsoDateLocal(weekStart)}T12:00:00`)),
    slots,
    slotCount: slots.length,
    replacementsThisWeek: replacementsThisWeek.length,
    live,
    dayContexts,
    schoolHolidayZone: zone,
    profScoped: Boolean(assignedClasses),
  });
}
