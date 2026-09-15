import { NextResponse } from "next/server";
import { requireInternatAccess } from "@/app/api/internat/_auth";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  filterInternatStudentsByViewerScope,
  isOrgAdminMetadata,
  resolveInternatRollCallViewerScope,
} from "@/app/lib/internat-rbac";
import { getInternatStudents, listRollCallHistory } from "@/app/lib/internat-storage";
import {
  INTERNAT_ROLL_MARK_LABELS,
  studentDisplayName,
  type InternatRollCallPeriod,
  type InternatRollCallRecipients,
} from "@/app/lib/internat-types";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

export async function GET(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  const { searchParams } = new URL(req.url);
  const from = searchParams.get("from") || undefined;
  const to = searchParams.get("to") || undefined;
  const studentId = searchParams.get("studentId") || undefined;
  const periodRaw = searchParams.get("period");
  const period: InternatRollCallPeriod | undefined =
    periodRaw === "matin" ? "matin" : periodRaw === "soir" ? "soir" : undefined;
  const limit = Math.min(120, Math.max(1, Number(searchParams.get("limit") || 60)));

  const bundle = await loadAppConfig();
  const notif = bundle.notifications as typeof bundle.notifications & {
    internatRollCallRecipients?: InternatRollCallRecipients;
  };
  const viewerScope = resolveInternatRollCallViewerScope({
    roles: access.roles,
    email: access.user?.primaryEmailAddress?.emailAddress,
    isOrgAdmin: isOrgAdminMetadata(access.user?.publicMetadata),
    recipients: notif.internatRollCallRecipients,
    establishments: bundle.establishments,
  });

  const [history, allStudents] = await Promise.all([
    listRollCallHistory({ from, to, studentId, period, limit }),
    getInternatStudents(),
  ]);

  const students = filterInternatStudentsByViewerScope(allStudents, viewerScope);
  const allowedIds = new Set(students.map((s) => s.id));
  const studentMap = new Map(students.map((s) => [s.id, s]));

  const rows = history.flatMap((day) =>
    day.marks
      .filter((m) => allowedIds.has(m.studentId))
      .map((m) => {
        const student = studentMap.get(m.studentId);
        return {
          date: day.date,
          period: day.period,
          studentId: m.studentId,
          studentName: student ? studentDisplayName(student) : m.studentId,
          classe: student?.classe,
          etablissement: student?.etablissement,
          etabKind: student
            ? inferEstablishmentKind({ label: student.etablissement })
            : undefined,
          mark: m.mark,
          markLabel: INTERNAT_ROLL_MARK_LABELS[m.mark as keyof typeof INTERNAT_ROLL_MARK_LABELS] || m.mark,
          validatedAt: day.validatedAt,
          validatedBy: day.validatedBy,
        };
      }),
  );

  return NextResponse.json({
    history: rows,
    students: students.filter((s) => s.actif),
    viewerScope,
  });
}
