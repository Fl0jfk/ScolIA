/** Pulse quotidien anonyme « Comment je me sens » — module RH. */

export const MOOD_PULSE_MIN = 1;
export const MOOD_PULSE_MAX = 10;
export const MOOD_PULSE_COMMENT_MAX = 280;
export const MOOD_PULSE_HISTORY_DAYS = 14;

export type MoodPulseScore = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10;

export type MoodPulseEntry = {
  id: string;
  score: MoodPulseScore;
  comment?: string;
  createdAt: string;
};

export type MoodPulseDayDoc = {
  version: 1;
  date: string;
  /** Hashes anonymes des votants du jour — jamais exposés en API RH. */
  voters: string[];
  entries: MoodPulseEntry[];
};

export type MoodPulseDayAggregate = {
  date: string;
  count: number;
  average: number | null;
  distribution: Record<MoodPulseScore, number>;
  comments: Array<{ id: string; score: MoodPulseScore; comment: string; createdAt: string }>;
};

export type MoodPulseCollabResponse = {
  date: string;
  submittedToday: boolean;
  canManage: boolean;
};

export type MoodPulseAdminResponse = MoodPulseCollabResponse & {
  today: MoodPulseDayAggregate;
  history: Array<{ date: string; count: number; average: number | null }>;
};

export function moodPulseDayKey(date: string) {
  return `rh/mood-pulse/${date}.json`;
}

export function emptyDistribution(): Record<MoodPulseScore, number> {
  return { 1: 0, 2: 0, 3: 0, 4: 0, 5: 0, 6: 0, 7: 0, 8: 0, 9: 0, 10: 0 };
}

export function isMoodPulseScore(n: unknown): n is MoodPulseScore {
  return typeof n === "number" && Number.isInteger(n) && n >= MOOD_PULSE_MIN && n <= MOOD_PULSE_MAX;
}

/** Émoticône selon la note (1→triste … 10→rayonnant). */
export function moodPulseEmoji(score: number | null | undefined): string {
  if (score == null || !Number.isFinite(score)) return "🙂";
  const n = Math.round(score);
  if (n <= 1) return "😭";
  if (n <= 2) return "😢";
  if (n <= 3) return "😞";
  if (n <= 4) return "😕";
  if (n <= 5) return "😐";
  if (n <= 6) return "🙂";
  if (n <= 7) return "😊";
  if (n <= 8) return "😄";
  if (n <= 9) return "😁";
  return "🤩";
}

export function moodPulseLabel(score: number | null | undefined): string {
  if (score == null || !Number.isFinite(score)) return "";
  const n = Math.round(score);
  if (n <= 2) return "Très difficile";
  if (n <= 4) return "Pas top";
  if (n <= 5) return "Moyen";
  if (n <= 7) return "Ça va";
  if (n <= 8) return "Bien";
  if (n <= 9) return "Très bien";
  return "Au top";
}
