import { NextResponse } from "next/server";
import { requireInternatAccess } from "@/app/api/internat/_auth";
import { resolvePhotoUrlsForInternatStudents } from "@/app/lib/eleve-photos";
import {
  getInternatRollCall,
  getInternatStudents,
  saveInternatRollCall,
} from "@/app/lib/internat-storage";
import { notifyInternatRollCallValidated } from "@/app/lib/internat-notify";
import { rollCallCanValidate, sectionIsComplete, todayDateParis } from "@/app/lib/internat-stats";
import type { InternatRollCall, InternatRollCallPeriod, InternatRollMark, InternatRollSection } from "@/app/lib/internat-types";

function parsePeriod(raw: string | null): InternatRollCallPeriod {
  return raw === "matin" ? "matin" : "soir";
}

function mergeMarks(
  current: Record<string, InternatRollMark>,
  patch: Record<string, string | null | undefined>,
): Record<string, InternatRollMark> {
  const next = { ...current };
  for (const [studentId, mark] of Object.entries(patch)) {
    if (mark === null || mark === undefined || mark === "" || mark === "clear") {
      delete next[studentId];
      continue;
    }
    if (mark === "present" || mark === "absent" || mark === "excuse" || mark === "activite") {
      next[studentId] = mark;
    }
  }
  return next;
}

function mergeSection(
  current: InternatRollSection,
  patch:
    | {
        marks?: Record<string, string | null>;
        completed?: boolean;
        completedBy?: string;
        completedAt?: string;
      }
    | undefined,
): InternatRollSection {
  if (!patch) return current;
  const marks = patch.marks ? mergeMarks(current.marks, patch.marks) : current.marks;
  const completed = patch.completed !== undefined ? patch.completed : current.completed;
  return {
    completed,
    completedBy: completed ? (patch.completedBy ?? current.completedBy) : patch.completedBy,
    completedAt: completed ? (patch.completedAt ?? current.completedAt) : patch.completedAt,
    marks,
  };
}

export async function GET(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  const { searchParams } = new URL(req.url);
  const date = String(searchParams.get("date") || todayDateParis());
  const period = parsePeriod(searchParams.get("period"));
  const [rollCall, students] = await Promise.all([getInternatRollCall(date, period), getInternatStudents()]);
  const photoUrls = await resolvePhotoUrlsForInternatStudents(students);

  return NextResponse.json({
    rollCall,
    students,
    photoUrls,
    canValidate: rollCallCanValidate(rollCall, students),
    boysComplete: sectionIsComplete(rollCall.boys, students, "M"),
    girlsComplete: sectionIsComplete(rollCall.girls, students, "F"),
  });
}

export async function PATCH(req: Request) {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => ({}));
  const date = String(body.date || todayDateParis());
  const period = parsePeriod(body.period ? String(body.period) : null);
  const students = await getInternatStudents();
  let rollCall = await getInternatRollCall(date, period);

  if (rollCall.status === "validee") {
    return NextResponse.json({ error: "Cet appel est déjà validé." }, { status: 400 });
  }

  const now = new Date().toISOString();

  if (body.section === "boys" || body.section === "girls") {
    const key: "boys" | "girls" = body.section;
    const sectionPatch: {
      marks?: Record<string, string | null>;
      completed?: boolean;
      completedBy?: string;
      completedAt?: string;
    } = {};
    if (body.marks && typeof body.marks === "object") {
      sectionPatch.marks = body.marks as Record<string, string | null>;
      if (rollCall[key].completed) {
        sectionPatch.completed = false;
        sectionPatch.completedBy = undefined;
        sectionPatch.completedAt = undefined;
      }
    }
    if (body.complete === true) {
      const sexe = key === "girls" ? "F" : "M";
      const active = students.filter((s) => s.actif && s.sexe === sexe);
      const merged = mergeSection(rollCall[key], sectionPatch);
      const missing = active.filter((s) => !merged.marks[s.id]);
      if (missing.length > 0) {
        return NextResponse.json(
          { error: "Marquez tous les internes de cette section avant de la terminer." },
          { status: 400 },
        );
      }
      sectionPatch.completed = true;
      sectionPatch.completedBy = access.userName;
      sectionPatch.completedAt = now;
    }
    rollCall = {
      ...rollCall,
      [key]: mergeSection(rollCall[key], sectionPatch),
      updatedAt: now,
    };
  }

  await saveInternatRollCall(rollCall);
  return NextResponse.json({
    rollCall,
    canValidate: rollCallCanValidate(rollCall, students),
    boysComplete: sectionIsComplete(rollCall.boys, students, "M"),
    girlsComplete: sectionIsComplete(rollCall.girls, students, "F"),
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
