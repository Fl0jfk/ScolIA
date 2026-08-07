"use client";

import { useCallback, useEffect, useState } from "react";
import {
  MOOD_PULSE_MAX,
  moodPulseEmoji,
  type MoodPulseAdminResponse,
  type MoodPulseScore,
} from "@/app/lib/rh/mood-pulse-types";

const SCORES = Array.from({ length: MOOD_PULSE_MAX }, (_, i) => (i + 1) as MoodPulseScore);

function formatDayLabel(dateKey: string) {
  try {
    return new Date(`${dateKey}T12:00:00`).toLocaleDateString("fr-FR", {
      weekday: "short",
      day: "numeric",
      month: "short",
    });
  } catch {
    return dateKey;
  }
}

export default function RhMoodPulseAdminPanel() {
  const [data, setData] = useState<MoodPulseAdminResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/rh/mood-pulse", { cache: "no-store" });
      const json = (await res.json()) as MoodPulseAdminResponse & { error?: string };
      if (!res.ok) throw new Error(json.error || "Chargement impossible");
      if (!json.today || !json.history) {
        setData(null);
        return;
      }
      setData(json);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading) {
    return (
      <section className="rounded-2xl border border-slate-100 bg-white p-5 shadow-sm">
        <p className="text-sm text-slate-400">Chargement du pulse RH…</p>
      </section>
    );
  }

  if (error) {
    return (
      <section className="rounded-2xl border border-rose-100 bg-rose-50/50 p-5 shadow-sm">
        <p className="text-sm font-medium text-rose-700">{error}</p>
      </section>
    );
  }

  if (!data?.today) return null;

  const { today, history } = data;
  const maxDist = Math.max(1, ...SCORES.map((s) => today.distribution[s]));
  const historyChrono = [...history].reverse();
  const maxHistCount = Math.max(1, ...historyChrono.map((h) => h.count));

  return (
    <section className="rounded-2xl border border-violet-100 bg-white p-5 shadow-sm space-y-5">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-[11px] font-black uppercase tracking-widest text-violet-700">
            Pulse collaborateur
          </p>
          <h3 className="mt-1 text-lg font-black text-slate-900">Comment ça va aujourd’hui</h3>
          <p className="mt-1 text-xs text-slate-500">
            Remontées anonymes — aucune identité n’est stockée avec la note.
          </p>
        </div>
        <div className="rounded-2xl border border-violet-100 bg-violet-50 px-4 py-3 text-center min-w-[7rem]">
          <p className="text-[10px] font-black uppercase tracking-wide text-violet-600">Moyenne</p>
          <p className="text-2xl leading-none" aria-hidden>
            {moodPulseEmoji(today.average)}
          </p>
          <p className="mt-1 text-2xl font-black text-violet-900 tabular-nums">
            {today.average != null ? today.average.toFixed(1) : "—"}
            <span className="text-sm font-bold text-violet-400">/10</span>
          </p>
          <p className="text-[11px] font-semibold text-violet-700 mt-0.5">
            {today.count} réponse{today.count > 1 ? "s" : ""}
          </p>
        </div>
      </header>

      <div>
        <p className="text-xs font-black uppercase tracking-wide text-slate-500 mb-2">
          Répartition du jour
        </p>
        <div className="flex items-end gap-1.5 h-24">
          {SCORES.map((s) => {
            const n = today.distribution[s];
            const h = Math.max(n > 0 ? 12 : 4, Math.round((n / maxDist) * 88));
            return (
              <div key={s} className="flex-1 flex flex-col items-center gap-1 min-w-0">
                <span className="text-[9px] font-bold tabular-nums text-slate-500">
                  {n > 0 ? n : ""}
                </span>
                <div
                  className="w-full max-w-[28px] rounded-t-md bg-violet-400/80"
                  style={{ height: h }}
                  title={`${n} × note ${s}`}
                />
                <span className="text-[10px] font-bold text-slate-600">{s}</span>
              </div>
            );
          })}
        </div>
      </div>

      <div>
        <p className="text-xs font-black uppercase tracking-wide text-slate-500 mb-2">
          14 derniers jours
        </p>
        <div className="flex items-end gap-1 h-16">
          {historyChrono.map((h) => {
            const bar = Math.max(h.count > 0 ? 6 : 2, Math.round((h.count / maxHistCount) * 56));
            return (
              <div
                key={h.date}
                className="flex-1 flex flex-col items-center gap-0.5 min-w-0"
                title={`${formatDayLabel(h.date)} · ${h.count} · moy. ${h.average ?? "—"}`}
              >
                <div
                  className={`w-full max-w-[18px] rounded-t ${
                    h.date === today.date ? "bg-violet-600" : "bg-slate-200"
                  }`}
                  style={{ height: bar }}
                />
              </div>
            );
          })}
        </div>
        <div className="mt-1 flex justify-between text-[10px] text-slate-400 font-medium">
          <span>{formatDayLabel(historyChrono[0]?.date || "")}</span>
          <span>aujourd’hui</span>
        </div>
      </div>

      <div>
        <p className="text-xs font-black uppercase tracking-wide text-slate-500 mb-2">
          Commentaires du jour ({today.comments.length})
        </p>
        {today.comments.length === 0 ? (
          <p className="text-sm text-slate-400 italic">Aucun commentaire aujourd’hui.</p>
        ) : (
          <ul className="space-y-2 max-h-56 overflow-y-auto">
            {today.comments.map((c) => (
              <li
                key={c.id}
                className="rounded-xl border border-slate-100 bg-slate-50/80 px-3 py-2.5"
              >
                <div className="flex items-center gap-2 mb-1">
                  <span className="inline-flex h-6 min-w-6 items-center justify-center gap-0.5 rounded-lg bg-violet-100 px-1.5 text-[11px] font-black text-violet-800">
                    <span aria-hidden>{moodPulseEmoji(c.score)}</span>
                    {c.score}/10
                  </span>
                  <span className="text-[10px] text-slate-400">
                    {new Date(c.createdAt).toLocaleTimeString("fr-FR", {
                      hour: "2-digit",
                      minute: "2-digit",
                    })}
                  </span>
                </div>
                <p className="text-sm text-slate-700 whitespace-pre-wrap">{c.comment}</p>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}
