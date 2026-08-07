"use client";

import { useCallback, useEffect, useState } from "react";
import {
  MOOD_PULSE_COMMENT_MAX,
  MOOD_PULSE_MAX,
  MOOD_PULSE_MIN,
  moodPulseEmoji,
  moodPulseLabel,
  type MoodPulseCollabResponse,
  type MoodPulseScore,
} from "@/app/lib/rh/mood-pulse-types";

const SCORES = Array.from({ length: MOOD_PULSE_MAX }, (_, i) => (i + 1) as MoodPulseScore);

export default function RhMoodPulseCard() {
  const [loading, setLoading] = useState(true);
  const [submittedToday, setSubmittedToday] = useState(false);
  const [score, setScore] = useState<MoodPulseScore | null>(null);
  const [comment, setComment] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [justSent, setJustSent] = useState(false);

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

  const submit = async () => {
    if (score == null) {
      setError("Choisissez une note de 1 à 10.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/rh/mood-pulse", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          score,
          comment: comment.trim() || undefined,
        }),
      });
      const json = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(json.error || "Envoi impossible");
      setSubmittedToday(true);
      setJustSent(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
        <p className="text-sm text-slate-400">Chargement du pulse…</p>
      </section>
    );
  }

  if (submittedToday) {
    return (
      <section className="rounded-2xl border border-emerald-100 bg-gradient-to-br from-emerald-50/80 to-white p-5 shadow-sm">
        <div className="flex items-start gap-3">
          <span className="text-3xl leading-none" aria-hidden>
            {justSent && score != null ? moodPulseEmoji(score) : "✅"}
          </span>
          <div className="min-w-0">
            <p className="text-[11px] font-black uppercase tracking-widest text-emerald-700">
              Comment je me sens
            </p>
            <h3 className="mt-1 text-lg font-black text-slate-900">
              {justSent ? "Merci, c’est noté." : "Déjà répondu aujourd’hui"}
            </h3>
            <p className="mt-1 text-sm text-slate-600">
              Votre note est anonyme. La RH voit uniquement des moyennes et commentaires sans votre
              identité.
            </p>
          </div>
        </div>
      </section>
    );
  }

  const previewEmoji = moodPulseEmoji(score);
  const previewLabel = moodPulseLabel(score);

  return (
    <section className="rounded-2xl border border-violet-100 bg-gradient-to-br from-violet-50/70 via-white to-white p-5 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="text-[11px] font-black uppercase tracking-widest text-violet-700">
            Pulse du jour
          </p>
          <h3 className="mt-1 text-lg font-black text-slate-900">Comment je me sens aujourd’hui ?</h3>
          <p className="mt-1 text-sm text-slate-600">
            Note anonyme sur 10 — une seule fois par jour. Optionnel : un mot pour la RH.
          </p>
        </div>
        <div
          className="flex h-14 w-14 shrink-0 flex-col items-center justify-center rounded-2xl border border-violet-100 bg-white shadow-sm transition-transform duration-200"
          style={{ transform: score != null ? "scale(1.06)" : "scale(1)" }}
          aria-hidden
        >
          <span className="text-3xl leading-none">{previewEmoji}</span>
        </div>
      </div>

      <div className="mt-4 flex flex-wrap gap-1.5">
        {SCORES.map((n) => {
          const active = score === n;
          return (
            <button
              key={n}
              type="button"
              onClick={() => setScore(n)}
              title={`${n}/10 ${moodPulseEmoji(n)}`}
              className={`relative flex h-11 w-11 flex-col items-center justify-center rounded-xl text-sm font-black transition ${
                active
                  ? "bg-violet-600 text-white shadow-sm"
                  : "border border-slate-200 bg-white text-slate-700 hover:border-violet-300 hover:bg-violet-50"
              }`}
              aria-pressed={active}
              aria-label={`Note ${n} sur ${MOOD_PULSE_MAX}, ${moodPulseLabel(n)}`}
            >
              <span className="text-[11px] leading-none opacity-80">{moodPulseEmoji(n)}</span>
              <span className="mt-0.5 leading-none">{n}</span>
            </button>
          );
        })}
      </div>
      <p className="mt-1.5 text-[10px] font-medium text-slate-400">
        {score != null ? (
          <>
            {previewEmoji} {previewLabel} · {score}/{MOOD_PULSE_MAX}
          </>
        ) : (
          <>
            {MOOD_PULSE_MIN} = très mal · {MOOD_PULSE_MAX} = très bien
          </>
        )}
      </p>

      <label className="mt-4 block">
        <span className="text-xs font-bold text-slate-600">Commentaire (optionnel)</span>
        <textarea
          value={comment}
          onChange={(e) => setComment(e.target.value.slice(0, MOOD_PULSE_COMMENT_MAX))}
          rows={2}
          maxLength={MOOD_PULSE_COMMENT_MAX}
          placeholder="Un ressenti libre, sans vous identifier…"
          className="mt-1.5 w-full resize-none rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-800 outline-none focus:border-violet-400 focus:ring-2 focus:ring-violet-100"
        />
        <span className="mt-1 block text-right text-[10px] text-slate-400">
          {comment.length}/{MOOD_PULSE_COMMENT_MAX}
        </span>
      </label>

      {error ? <p className="mt-2 text-sm font-medium text-rose-600">{error}</p> : null}

      <button
        type="button"
        disabled={saving || score == null}
        onClick={() => void submit()}
        className="mt-3 rounded-xl bg-violet-600 px-4 py-2.5 text-xs font-bold text-white shadow-sm transition hover:bg-violet-700 disabled:opacity-50"
      >
        {saving ? "Envoi…" : "Envoyer anonymement"}
      </button>
    </section>
  );
}
