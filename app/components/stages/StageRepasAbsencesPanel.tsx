"use client";

import { useCallback, useEffect, useState } from "react";

type AbsenceRow = {
  conventionId: string;
  studentName: string;
  className: string;
  companyName: string;
  date: string;
  status: string;
};

type DayBucket = { date: string; absences: AbsenceRow[] };

export default function StageRepasAbsencesPanel({
  onOpenConvention,
}: {
  onOpenConvention?: (conventionId: string) => void;
}) {
  const [days, setDays] = useState<DayBucket[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const qs = new URLSearchParams();
      if (from) qs.set("from", from);
      if (to) qs.set("to", to);
      const res = await fetch(`/api/stages/repas-absences?${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setDays(data.days || []);
      if (!from) setFrom(data.from || "");
      if (!to) setTo(data.to || "");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [from, to]);

  useEffect(() => {
    void load();
  }, []); // premier chargement

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-3">
        <label className="text-xs font-semibold text-stone-600">
          Du
          <input
            type="date"
            className="mt-1 block rounded-lg border px-2 py-1.5 text-sm"
            value={from}
            onChange={(e) => setFrom(e.target.value)}
          />
        </label>
        <label className="text-xs font-semibold text-stone-600">
          Au
          <input
            type="date"
            className="mt-1 block rounded-lg border px-2 py-1.5 text-sm"
            value={to}
            onChange={(e) => setTo(e.target.value)}
          />
        </label>
        <button
          type="button"
          onClick={() => void load()}
          className="rounded-lg bg-[#2F6B4A] px-3 py-2 text-sm font-semibold text-white"
        >
          Actualiser
        </button>
      </div>

      {loading && <p className="text-sm text-stone-500">Chargement…</p>}
      {error && <p className="text-sm text-rose-700">{error}</p>}

      {!loading && days.length === 0 && (
        <p className="rounded-lg border border-stone-200 bg-stone-50 px-3 py-2 text-sm text-stone-600">
          Aucune absence stage (repas) sur la période.
        </p>
      )}

      <div className="space-y-3">
        {days.map((day) => (
          <div key={day.date} className="rounded-xl border border-amber-200 bg-amber-50/60 p-4">
            <h3 className="text-sm font-bold capitalize text-amber-950">
              {new Date(`${day.date}T12:00:00`).toLocaleDateString("fr-FR", {
                weekday: "long",
                day: "numeric",
                month: "long",
              })}
              <span className="ml-2 font-normal text-amber-800/80">
                — {day.absences.length} élève{day.absences.length > 1 ? "s" : ""}
              </span>
            </h3>
            <ul className="mt-2 divide-y divide-amber-100">
              {day.absences.map((a) => (
                <li
                  key={`${a.conventionId}-${a.date}`}
                  className="flex flex-wrap items-baseline gap-2 py-1.5 text-sm"
                >
                  {onOpenConvention ? (
                    <button
                      type="button"
                      onClick={() => onOpenConvention(a.conventionId)}
                      className="font-semibold text-[#2F6B4A] underline hover:no-underline"
                    >
                      {a.studentName}
                    </button>
                  ) : (
                    <span className="font-semibold text-stone-900">{a.studentName}</span>
                  )}
                  <span className="text-stone-500">{a.className}</span>
                  <span className="text-xs text-stone-500">· {a.companyName}</span>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}
