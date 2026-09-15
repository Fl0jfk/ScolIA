import { NextResponse } from "next/server";
import { requireInternatAccess } from "@/app/api/internat/_auth";
import { loadAppConfig } from "@/app/lib/app-config";
import type {
  InternatRollCall,
  InternatRollCallPeriod,
  InternatRollCallRecipients,
  InternatRollMark,
  InternatRollMarkHistoryEntry,
  InternatRollMarkMeta,
  InternatRollSection,
} from "@/app/lib/internat-types";
import { resolvePhotoUrlsForInternatStudents } from "@/app/lib/eleve-photos";
import { buildInternatCourseAbsenceHints } from "@/app/lib/internat-course-absences";
import {
  filterInternatStudentsByViewerScope,
  isOrgAdminMetadata,
  resolveInternatRollCallViewerScope,
} from "@/app/lib/internat-rbac";
import {
  getInternatRollCall,
  getInternatStudents,
  saveInternatRollCall,
} from "@/app/lib/internat-storage";
import {
  notifyInternatRollCallCorrection,
  notifyInternatRollCallValidated,
} from "@/app/lib/internat-notify";
import { rollCallCanValidate, sectionIsComplete, todayDateParis } from "@/app/lib/internat-stats";

function parsePeriod(raw: string | null): InternatRollCallPeriod {
  return raw === "matin" ? "matin" : "soir";
}

function parseMark(raw: unknown): InternatRollMark | null | undefined {
  if (raw === null || raw === "" || raw === "clear") return null;
  if (raw === "present" || raw === "absent" || raw === "excuse" || raw === "activite") {
    return raw;
  }
  return undefined;
}

function applyMarkPatch(
  section: InternatRollSection,
  patch: Record<string, string | null | undefined>,
  meta: { at: string; by: string; note?: string },
): {
  section: InternatRollSection;
  history: InternatRollMarkHistoryEntry[];
} {
  const marks = { ...section.marks };
  const markMeta: Record<string, InternatRollMarkMeta> = { ...(section.markMeta || {}) };
  const history: InternatRollMarkHistoryEntry[] = [];

  for (const [studentId, raw] of Object.entries(patch)) {
    const parsed = parseMark(raw);
    if (parsed === undefined) continue;
    const previous = marks[studentId];
    if (parsed === null) {
      delete marks[studentId];
      delete markMeta[studentId];
    } else {
      marks[studentId] = parsed;
      markMeta[studentId] = {
        at: meta.at,
        by: meta.by,
        note: meta.note,
        previous,
      };
    }
    history.push({
      studentId,
      at: meta.at,
      by: meta.by,
      mark: parsed,
      previous,
      note: meta.note,
    });
  }

  return {
    section: {
      ...section,
      marks,
      markMeta,
    },
    history,
  };
}

async function resolveViewerScope(access: {
  roles: string[];
  user: { primaryEmailAddress?: { emailAddress?: string } | null; publicMetadata?: unknown } | null;
}) {
  const bundle = await loadAppConfig();
  const notif = bundle.notifications as typeof bundle.notifications & {
    internatRollCallRecipients?: InternatRollCallRecipients;
  };
  return {
    viewerScope: resolveInternatRollCallViewerScope({
      roles: access.roles,
      email: access.user?.primaryEmailAddress?.emailAddress,
      isOrgAdmin: isOrgAdminMetadata(access.user?.publicMetadata),
      recipients: notif.internatRollCallRecipients,
      establishments: bundle.establishments,
    }),
    establishments: bundle.establishments,
  };
}

export async function GET(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  try {
    const { searchParams } = new URL(req.url);
    const date = String(searchParams.get("date") || todayDateParis());
    const period = parsePeriod(searchParams.get("period"));
    const [{ viewerScope, establishments }, rollCall, allStudents] = await Promise.all([
      resolveViewerScope(access),
      getInternatRollCall(date, period),
      getInternatStudents(),
    ]);
    const students = filterInternatStudentsByViewerScope(allStudents, viewerScope, establishments);

    const [photoUrls, courseAbsenceHints] = await Promise.all([
      resolvePhotoUrlsForInternatStudents(students).catch((e) => {
        console.warn("[internat/roll-call] photoUrls", e);
        return {} as Record<string, string>;
      }),
      buildInternatCourseAbsenceHints(date, students),
    ]);

    return NextResponse.json({
      rollCall,
      students,
      viewerScope,
      photoUrls,
      courseAbsenceHints,
      canValidate: viewerScope === "all" && rollCallCanValidate(rollCall, allStudents),
      boysComplete: sectionIsComplete(rollCall.boys, allStudents, "M"),
      girlsComplete: sectionIsComplete(rollCall.girls, allStudents, "F"),
    });
  } catch (e) {
    console.error("[internat/roll-call] GET", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Chargement de l'appel impossible." },
      { status: 500 },
    );
  }
}

export async function PATCH(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => ({}));
  const date = String(body.date || todayDateParis());
  const period = parsePeriod(body.period ? String(body.period) : null);
  const students = await getInternatStudents();
  let rollCall = await getInternatRollCall(date, period);
  const now = new Date().toISOString();
  const note = String(body.note || "").trim() || undefined;
  const afterValidation = rollCall.status === "validee";

  if (afterValidation && body.complete === true) {
    return NextResponse.json(
      { error: "Impossible de terminer une section sur un appel déjà validé." },
      { status: 400 },
    );
  }

  if (afterValidation) {
    if (!body.marks || typeof body.marks !== "object") {
      return NextResponse.json(
        { error: "Sur un appel validé, seules des corrections de statut sont possibles." },
        { status: 400 },
      );
    }
    if (!note) {
      return NextResponse.json(
        {
          error:
            "Précisez le motif (ex. activité sportive — arrivé à 21h10) pour corriger un appel déjà validé.",
        },
        { status: 400 },
      );
    }
  }

  const correctionMails: Array<{ studentId: string; sent: boolean }> = [];

  if (body.section === "boys" || body.section === "girls") {
    const key: "boys" | "girls" = body.section;
    let nextSection = rollCall[key];
    const historyAdds: InternatRollMarkHistoryEntry[] = [];

    if (body.marks && typeof body.marks === "object") {
      const applied = applyMarkPatch(nextSection, body.marks as Record<string, string | null>, {
        at: now,
        by: access.userName,
        note,
      });
      nextSection = applied.section;
      for (const h of applied.history) {
        historyAdds.push({ ...h, afterValidation: afterValidation || undefined });
      }

      if (!afterValidation && nextSection.completed) {
        nextSection = {
          ...nextSection,
          completed: false,
          completedBy: undefined,
          completedAt: undefined,
        };
      }
    }

    if (!afterValidation && body.complete === true) {
      const sexe = key === "girls" ? "F" : "M";
      const active = students.filter((s) => s.actif && s.sexe === sexe);
      const missing = active.filter((s) => !nextSection.marks[s.id]);
      if (missing.length > 0) {
        return NextResponse.json(
          { error: "Marquez tous les internes de cette section avant de la terminer." },
          { status: 400 },
        );
      }
      nextSection = {
        ...nextSection,
        completed: true,
        completedBy: access.userName,
        completedAt: now,
      };
    }

    rollCall = {
      ...rollCall,
      [key]: nextSection,
      markHistory: [...(rollCall.markHistory || []), ...historyAdds],
      updatedAt: now,
    };

    if (afterValidation && historyAdds.length > 0) {
      for (const entry of historyAdds) {
        if (!entry.mark) continue;
        const student = students.find((s) => s.id === entry.studentId);
        if (!student) continue;
        const mail = await notifyInternatRollCallCorrection({
          rollCall,
          student,
          mark: entry.mark,
          previous: entry.previous,
          note: entry.note,
          correctedBy: access.userName,
          correctedAt: now,
        });
        correctionMails.push({ studentId: student.id, sent: mail.sent });
      }
    }
  }

  await saveInternatRollCall(rollCall);
  return NextResponse.json({
    rollCall,
    canValidate: !afterValidation && rollCallCanValidate(rollCall, students),
    boysComplete: sectionIsComplete(rollCall.boys, students, "M"),
    girlsComplete: sectionIsComplete(rollCall.girls, students, "F"),
    correctionMails,
  });
}

export async function POST(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => ({}));
  if (body.action !== "validate") {
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  }

  const date = String(body.date || todayDateParis());
  const period = parsePeriod(body.period ? String(body.period) : null);
  const students = await getInternatStudents();
  const rollCall = await getInternatRollCall(date, period);

  if (!rollCallCanValidate(rollCall, students)) {
    return NextResponse.json(
      { error: "Les sections garçons et filles doivent être complètes avant validation." },
      { status: 400 },
    );
  }

  const now = new Date().toISOString();
  const validated: InternatRollCall = {
    ...rollCall,
    status: "validee",
    validatedAt: now,
    validatedBy: access.userName,
    updatedAt: now,
  };

  const mail = await notifyInternatRollCallValidated({
    rollCall: validated,
    students,
    validatedBy: access.userName,
  });

  if (mail.sent) validated.emailSentAt = now;

  await saveInternatRollCall(validated);
  return NextResponse.json({ rollCall: validated, mail });
}
