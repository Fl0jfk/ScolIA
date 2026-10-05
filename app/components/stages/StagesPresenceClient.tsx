"use client";

import { useCallback, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";

type PresenceRow = {
  conventionId: string;
  eleveId: string;
  companyName: string;
  periodStart: string;
  periodEnd: string;
  status: string;
  studentName: string;
  className: string;
  occupancyTag: string;
  excludeFromBulletin: boolean;
};

function todayParis(): string {
  return new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

function normalizeIsoDate(raw: string | null): string | null {
  if (!raw) return null;
  const m = /^(\d{4}-\d{2}-\d{2})$/.exec(raw.trim());
  return m ? m[1]! : null;
}

export default function StagesPresenceClient() {
  const searchParams = useSearchParams();
  const dateFromUrl = normalizeIsoDate(searchParams.get("date"));
  const [date, setDate] = useState(dateFromUrl ?? todayParis);
  const [eleves, setEleves] = useState<PresenceRow[]>([]);
  const [unmatched, setUnmatched] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (dateFromUrl && dateFromUrl !== date) setDate(dateFromUrl);
  }, [dateFromUrl, date]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const res = await fetch(`/api/stages/presence?date=${encodeURIComponent(date)}`, {
      credentials: "include",
    });
    const data = (await res.json().catch(() => ({}))) as {
      error?: string;
      eleves?: PresenceRow[];
      unmatched?: number;
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      setEleves([]);
      setLoading(false);
      return;
    }
    setEleves(data.eleves ?? []);
    setUnmatched(data.unmatched ?? 0);
    setLoading(false);
  }, [date]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="space-y-6">
      <p className="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
        Élèves en stage aujourd’hui = conventions actives (période). Tag occupancy{" "}
        <code className="text-xs">en_stage</code> — <strong>pas</strong> une absence bulletin
        (comme une sortie).
      </p>

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <label className="text-sm font-semibold text-slate-800">
          Date
          <input
            type="date"
            value={date}
            lang="fr-CA"
            onChange={(e) => {
              const v = normalizeIsoDate(e.target.value);
              if (v) setDate(v);
            }}
            className="mt-1 block rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
          />
        </label>
        <button
          type="button"
          onClick={() => void load()}
          className="mt-3 rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-800"
        >
          Actualiser
        </button>
      </section>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-900">
          {error}
        </p>
      ) : null}

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Synthèse — {eleves.length} en stage
          {unmatched > 0 ? ` · ${unmatched} convention(s) sans élève lié` : ""}
        </h2>
        {loading ? (
          <p className="mt-3 text-sm text-slate-500">Chargement…</p>
        ) : eleves.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">
            Aucune convention active sur cette date (statut signé / signatures / déposé).
          </p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {eleves.map((e) => (
              <li
                key={e.conventionId}
                className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm"
              >
                <div>
                  <p className="font-semibold text-slate-900">
                    {e.studentName}
                    {e.className ? (
                      <span className="ml-2 text-xs font-medium text-slate-500">{e.className}</span>
                    ) : null}
                  </p>
                  <p className="text-xs text-slate-500">
                    {e.companyName} · {e.periodStart} → {e.periodEnd} · {e.status}
                  </p>
                </div>
                <span className="rounded-full border border-violet-200 bg-violet-50 px-2.5 py-1 text-xs font-bold text-violet-950">
                  en_stage · hors bulletin
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
