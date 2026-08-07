import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import {
  aggregateMoodPulseDay,
  hasVotedMoodPulse,
  listMoodPulseHistory,
  moodPulseTodayKey,
  readMoodPulseDay,
  submitMoodPulse,
} from "@/app/lib/rh/mood-pulse-storage";
import type {
  MoodPulseAdminResponse,
  MoodPulseCollabResponse,
} from "@/app/lib/rh/mood-pulse-types";

function rolesFromUser(user: NonNullable<Awaited<ReturnType<typeof safeCurrentUser>>>) {
  const rolesRaw = user?.publicMetadata?.role;
  return Array.isArray(rolesRaw) ? rolesRaw.map(String) : rolesRaw ? [String(rolesRaw)] : [];
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const roles = rolesFromUser(user);
  const canManage = canManagePersonnel(roles);
  const date = moodPulseTodayKey();
  const todayDoc = await readMoodPulseDay(date);
  const submittedToday = hasVotedMoodPulse(todayDoc, gate.ctx.userId);

  const base: MoodPulseCollabResponse = {
    date,
    submittedToday,
    canManage,
  };

  if (!canManage) {
    return NextResponse.json(base);
  }

  const today = aggregateMoodPulseDay(todayDoc);
  const history = await listMoodPulseHistory(date);
  const body: MoodPulseAdminResponse = {
    ...base,
    today,
    history,
  };
  return NextResponse.json(body);
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  let body: { score?: unknown; comment?: unknown };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const result = await submitMoodPulse({
    userId: gate.ctx.userId,
    score: body.score,
    comment: body.comment,
  });

  if (!result.ok) {
    const status = result.code === "ALREADY_SUBMITTED" ? 409 : 400;
    return NextResponse.json({ error: result.error, code: result.code }, { status });
  }

  return NextResponse.json({ ok: true, submittedToday: true });
}
