"use client";

import { useCallback, useEffect, useState } from "react";
import {
  MOOD_PULSE_MAX,
  moodPulseEmoji,
  type MoodPulseCollabResponse,
  type MoodPulseScore,
} from "@/app/lib/rh/mood-pulse-types";

const SCORES = Array.from({ length: MOOD_PULSE_MAX }, (_, i) => (i + 1) as MoodPulseScore);

export default function RhMoodPulseCard() {
  const [loading, setLoading] = useState(true);
  const [submittedToday, setSubmittedToday] = useState(false);
  const [score, setScore] = useState<MoodPulseScore | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/rh/mood-pulse", { cache: "no-store" });
      const json = (await res.json()) as MoodPulseCollabResponse & { error?: string };
      if (!res.ok) throw new Error(json.error || "Chargement impossible");
      setSubmittedToday(Boolean(json.submittedToday));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const submit = async (picked: MoodPulseScore) => {
    if (saving || submittedToday) return;
    setScore(picked);
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/rh/mood-pulse", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ score: picked }),
      });
      const json = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(json.error || "Envoi impossible");
      setSubmittedToday(true);
    } catch (e) {
      setScore(null);
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="rounded-xl border border-slate-200/80 bg-white/80 px-3 py-2.5 text-xs text-slate-400">
        Pulse…
      </div>
    );
  }

  if (submittedToday) {
    return (
      <div className="rounded-xl border border-emerald-100 bg-emerald-50/60 px-3 py-2.5">
        <div className="flex items-center gap-2">
          <span className="text-lg leading-none" aria-hidden>
            {score != null ? moodPulseEmoji(score) : "✓"}
          </span>
          <div className="min-w-0">
            <p className="text-[10px] font-black uppercase tracking-wide text-emerald-700">Pulse du jour</p>
            <p className="text-xs font-bold text-slate-800">Merci, c&apos;est noté.</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-violet-100 bg-white/90 px-3 py-2.5 shadow-sm">
      <div className="flex items-center gap-2 mb-2">
        <span className="text-lg leading-none shrink-0" aria-hidden>
          {score != null ? moodPulseEmoji(score) : "🙂"}
        </span>
        <p className="text-[11px] font-bold text-slate-800 leading-tight">
          Comment tu te sens aujourd&apos;hui ?
        </p>
      </div>

      <div className="grid grid-cols-5 gap-1">
        {SCORES.map((n) => {
          const active = score === n;
          return (
            <button
              key={n}
              type="button"
              disabled={saving}
              onClick={() => void submit(n)}
              title={`${n}/10`}
              className={`h-7 rounded-md text-[11px] font-black transition ${
                active
                  ? "bg-violet-600 text-white"
                  : "border border-slate-200 bg-slate-50 text-slate-700 hover:border-violet-300 hover:bg-violet-50"
              } disabled:opacity-50`}
              aria-label={`Note ${n} sur ${MOOD_PULSE_MAX}`}
            >
              {n}
            </button>
          );
        })}
      </div>

      {saving ? <p className="mt-1.5 text-[10px] text-slate-400">Envoi…</p> : null}
      {error ? <p className="mt-1.5 text-[10px] font-medium text-rose-600">{error}</p> : null}
    </div>
  );
}
