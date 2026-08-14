import "server-only";

import { createHash } from "crypto";
import { getJson, putJson } from "@/app/lib/s3-storage";

const LIMITS_PREFIX = "bien-etre/limits";

type BienEtreRateState = {
  messagesHour: number;
  hourWindowStart: string;
  signalementsDay: number;
  dayWindowStart: string;
};

const MAX_MESSAGES_PER_HOUR = 30;
const MAX_SIGNALEMENTS_PER_DAY = 2;

function hashUserId(userId: string): string {
  return createHash("sha256").update(userId).digest("hex").slice(0, 32);
}

function limitKey(userId: string): string {
  return `${LIMITS_PREFIX}/${hashUserId(userId)}.json`;
}

function hourStartIso(): string {
  const d = new Date();
  d.setMinutes(0, 0, 0);
  return d.toISOString();
}

function dayStartIso(): string {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d.toISOString();
}

async function loadState(userId: string): Promise<BienEtreRateState> {
  const raw = await getJson<BienEtreRateState>(limitKey(userId));
  if (!raw?.data) {
    return {
      messagesHour: 0,
      hourWindowStart: hourStartIso(),
      signalementsDay: 0,
      dayWindowStart: dayStartIso(),
    };
  }
  return raw.data;
}

async function saveState(userId: string, state: BienEtreRateState): Promise<void> {
  await putJson(limitKey(userId), state);
}

export async function checkAndIncrementMessageRate(
  userId: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const state = await loadState(userId);
  const hourStart = hourStartIso();
  if (state.hourWindowStart !== hourStart) {
    state.messagesHour = 0;
    state.hourWindowStart = hourStart;
  }
  if (state.messagesHour >= MAX_MESSAGES_PER_HOUR) {
    return {
      ok: false,
      error: "Tu as envoyé beaucoup de messages. Fais une pause et reviens dans un moment.",
    };
  }
  state.messagesHour += 1;
  await saveState(userId, state);
  return { ok: true };
}

export async function checkAndIncrementSignalementRate(
  userId: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const state = await loadState(userId);
  const dayStart = dayStartIso();
  if (state.dayWindowStart !== dayStart) {
    state.signalementsDay = 0;
    state.dayWindowStart = dayStart;
  }
  if (state.signalementsDay >= MAX_SIGNALEMENTS_PER_DAY) {
    return {
      ok: false,
      error: "Tu as déjà envoyé un signalement aujourd'hui. Le psychologue te recontactera si besoin.",
    };
  }
  state.signalementsDay += 1;
  await saveState(userId, state);
  return { ok: true };
}
