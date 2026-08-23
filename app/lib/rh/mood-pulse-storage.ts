import "server-only";

import { createHash } from "crypto";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { rhUid } from "@/app/lib/rh/types";
import {
  emptyDistribution,
  isMoodPulseScore,
  moodPulseDayKey,
  MOOD_PULSE_COMMENT_MAX,
  MOOD_PULSE_HISTORY_DAYS,
  type MoodPulseDayAggregate,
  type MoodPulseDayDoc,
  type MoodPulseEntry,
  type MoodPulseScore,
} from "@/app/lib/rh/mood-pulse-types";

function voterPepper() {
  return (
    process.env.RH_MOOD_PULSE_PEPPER?.trim() ||
    process.env.BETTER_AUTH_SECRET?.trim() ||
    "rh-mood-pulse-dev"
  );
}

export function moodPulseTodayKey(d = new Date()) {
  return calendarDateKeyParis(d);
}

function hashMoodPulseVoter(userId: string, date: string) {
  return createHash("sha256")
    .update(`${userId}|${date}|${voterPepper()}`)
    .digest("hex");
}

function emptyDay(date: string): MoodPulseDayDoc {
  return { version: 1, date, voters: [], entries: [] };
}

function normalizeDay(raw: unknown, date: string): MoodPulseDayDoc {
  if (!raw || typeof raw !== "object") return emptyDay(date);
  const o = raw as Partial<MoodPulseDayDoc>;
  const voters = Array.isArray(o.voters)
    ? o.voters.filter((v): v is string => typeof v === "string" && v.length > 0)
    : [];
  const entries: MoodPulseEntry[] = [];
  if (Array.isArray(o.entries)) {
    for (const e of o.entries) {
      if (!e || typeof e !== "object") continue;
      const row = e as Partial<MoodPulseEntry>;
      if (!isMoodPulseScore(row.score)) continue;
      const id = typeof row.id === "string" && row.id ? row.id : rhUid("mood");
      const createdAt =
        typeof row.createdAt === "string" && row.createdAt
          ? row.createdAt
          : new Date().toISOString();
      const comment =
        typeof row.comment === "string" && row.comment.trim()
          ? row.comment.trim().slice(0, MOOD_PULSE_COMMENT_MAX)
          : undefined;
      entries.push({ id, score: row.score, comment, createdAt });
    }
  }
  return { version: 1, date, voters, entries };
}

export async function readMoodPulseDay(date: string): Promise<MoodPulseDayDoc> {
  const hit = await getJson<unknown>(moodPulseDayKey(date));
  return normalizeDay(hit?.data, date);
}

async function writeMoodPulseDay(doc: MoodPulseDayDoc): Promise<void> {
  await putJson(moodPulseDayKey(doc.date), doc);
}

export function hasVotedMoodPulse(doc: MoodPulseDayDoc, userId: string) {
  const hash = hashMoodPulseVoter(userId, doc.date);
  return doc.voters.includes(hash);
}

export function aggregateMoodPulseDay(doc: MoodPulseDayDoc): MoodPulseDayAggregate {
  const distribution = emptyDistribution();
  let sum = 0;
  const comments: MoodPulseDayAggregate["comments"] = [];
  for (const e of doc.entries) {
    distribution[e.score] += 1;
    sum += e.score;
    if (e.comment) {
      comments.push({
        id: e.id,
        score: e.score,
        comment: e.comment,
        createdAt: e.createdAt,
      });
    }
  }
  comments.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  const count = doc.entries.length;
  return {
    date: doc.date,
    count,
    average: count > 0 ? Math.round((sum / count) * 10) / 10 : null,
    distribution,
    comments,
  };
}

function shiftCalendarDate(dateKey: string, deltaDays: number): string {
  const [y, m, d] = dateKey.split("-").map(Number);
  const dt = new Date(Date.UTC(y!, (m ?? 1) - 1, (d ?? 1) + deltaDays, 12, 0, 0));
  return dt.toISOString().slice(0, 10);
}

export async function listMoodPulseHistory(
  endDate: string,
  days = MOOD_PULSE_HISTORY_DAYS,
): Promise<Array<{ date: string; count: number; average: number | null }>> {
  const dates = Array.from({ length: days }, (_, i) => shiftCalendarDate(endDate, -i));
  const docs = await Promise.all(dates.map((date) => readMoodPulseDay(date)));
  return docs.map((doc, i) => {
    const agg = aggregateMoodPulseDay(doc);
    return { date: dates[i]!, count: agg.count, average: agg.average };
  });
}

type SubmitMoodPulseResult =
  | { ok: true; entry: MoodPulseEntry }
  | { ok: false; error: string; code: "ALREADY_SUBMITTED" | "INVALID_SCORE" | "INVALID_COMMENT" };

export async function submitMoodPulse(input: {
  userId: string;
  score: unknown;
  comment?: unknown;
  date?: string;
}): Promise<SubmitMoodPulseResult> {
  if (!isMoodPulseScore(input.score)) {
    return { ok: false, error: "La note doit être un entier entre 1 et 10.", code: "INVALID_SCORE" };
  }
  let comment: string | undefined;
  if (input.comment != null && input.comment !== "") {
    if (typeof input.comment !== "string") {
      return { ok: false, error: "Commentaire invalide.", code: "INVALID_COMMENT" };
    }
    const trimmed = input.comment.trim();
    if (trimmed.length > MOOD_PULSE_COMMENT_MAX) {
      return {
        ok: false,
        error: `Commentaire trop long (max ${MOOD_PULSE_COMMENT_MAX} caractères).`,
        code: "INVALID_COMMENT",
      };
    }
    comment = trimmed || undefined;
  }

  const date = input.date || moodPulseTodayKey();
  const doc = await readMoodPulseDay(date);
  const voterHash = hashMoodPulseVoter(input.userId, date);
  if (doc.voters.includes(voterHash)) {
    return { ok: false, error: "Vous avez déjà répondu aujourd’hui.", code: "ALREADY_SUBMITTED" };
  }

  const entry: MoodPulseEntry = {
    id: rhUid("mood"),
    score: input.score as MoodPulseScore,
    comment,
    createdAt: new Date().toISOString(),
  };

  doc.voters.push(voterHash);
  doc.entries.push(entry);
  await writeMoodPulseDay(doc);
  return { ok: true, entry };
}
