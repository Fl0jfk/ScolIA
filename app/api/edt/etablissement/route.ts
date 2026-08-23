import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { loadSchoolRoster } from "@/app/lib/school-roster";
import { requireAuth } from "@/app/lib/intranet-auth";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  canManagePersonnel,
  canViewPersonnelDashboard,
} from "@/app/lib/personnel-types";
import { findAllCrossTeacherPlanningConflicts } from "@/app/lib/rh/planning-conflicts";
import { listClassesFromTeacherIndex } from "@/app/lib/rh/planning-class-schedule";
import { getTeacherPlanningEntries } from "@/app/lib/rh/planning-teacher-index";

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
  if (!canManage && !canViewAll) {
    return NextResponse.json({ error: "Réservé à la direction / vie scolaire." }, { status: 403 });
  }

  const url = new URL(req.url);
  const weekType = url.searchParams.get("week") === "B" ? "B" : "A";

  const entries = await getTeacherPlanningEntries();
  const allConflicts = findAllCrossTeacherPlanningConflicts(entries);
  const conflicts =
    weekType === "A"
      ? allConflicts.filter((c) => c.weekType === "A")
      : allConflicts.filter((c) => c.weekType === "B");

  const cfg = await loadAppConfig();
  const roster = await loadSchoolRoster();
  const classesFromRoster = [
    ...new Set(roster.classAssignments.map((a) => a.className.trim()).filter(Boolean)),
  ];
  const classesFromProfRoom = Object.values(cfg.profRoom.classesByPole || {}).flat();
  const classesFromEdt = listClassesFromTeacherIndex(entries);
  const classes = [...new Set([...classesFromEdt, ...classesFromRoster, ...classesFromProfRoom])].sort(
    (a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }),
  );

  const teachersWithEdt = entries.length;
  const roomConflicts = conflicts.filter((c) => c.kind === "room").length;
  const classConflicts = conflicts.filter((c) => c.kind === "class").length;

  return NextResponse.json({
    weekType,
    teachersWithEdt,
    classCount: classes.length,
    classes,
    conflicts,
    summary: {
      total: conflicts.length,
      room: roomConflicts,
      class: classConflicts,
    },
  });
}
