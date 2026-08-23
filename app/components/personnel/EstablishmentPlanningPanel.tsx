"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import { dash } from "@/app/lib/dashboard-brand";

type Conflict = {
  kind: "room" | "class";
  weekType: "A" | "B";
  day: number;
  start: string;
  end: string;
  message: string;
  classes: string[];
  room: string | null;
};

type ApiPayload = {
  weekType: "A" | "B";
  teachersWithEdt: number;
  classCount: number;
  classes: string[];
  conflicts: Conflict[];
  summary: { total: number; room: number; class: number };
};

export default function EstablishmentPlanningPanel() {
  const [weekType, setWeekType] = useState<"A" | "B">("A");
  const [data, setData] = useState<ApiPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/edt/etablissement?week=${weekType}`);
      const j = (await res.json()) as ApiPayload & { error?: string };
      if (!res.ok) throw new Error(j.error || `Erreur ${res.status}`);
      setData(j);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [weekType]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center gap-3">
        {(["A", "B"] as const).map((id) => (
          <button
            key={id}
            type="button"
            onClick={() => setWeekType(id)}
            className={`px-3 py-2 rounded-lg text-xs font-bold ${
              weekType === id
                ? "bg-slate-900 text-white"
                : "bg-white border border-slate-200 text-slate-600"
            }`}
          >
            Semaine type {id}
          </button>
        ))}
        <Link
          href="/edt-classe"
          className="ml-auto text-xs font-bold text-indigo-600 hover:underline"
        >
          Vue par classe →
        </Link>
      </div>

      {error ? (
        <p className="text-sm text-rose-600 rounded-xl bg-rose-50 px-3 py-2">{error}</p>
      ) : null}

      {loading ? <p className={`text-sm ${dash.textMid}`}>Analyse de l’établissement…</p> : null}

      {data ? (
        <>
          <div className="grid gap-3 sm:grid-cols-4">
            <ModuleCard bodyClassName="p-4">
              <p className="text-[10px] font-black uppercase tracking-wide text-slate-500">Profs EDT</p>
              <p className="mt-1 text-2xl font-bold text-slate-900">{data.teachersWithEdt}</p>
            </ModuleCard>
            <ModuleCard bodyClassName="p-4">
              <p className="text-[10px] font-black uppercase tracking-wide text-slate-500">Classes</p>
              <p className="mt-1 text-2xl font-bold text-slate-900">{data.classCount}</p>
            </ModuleCard>
            <ModuleCard bodyClassName="p-4">
              <p className="text-[10px] font-black uppercase tracking-wide text-amber-600">Conflits salle</p>
              <p className="mt-1 text-2xl font-bold text-amber-900">{data.summary.room}</p>
            </ModuleCard>
            <ModuleCard bodyClassName="p-4">
              <p className="text-[10px] font-black uppercase tracking-wide text-rose-600">Conflits classe</p>
              <p className="mt-1 text-2xl font-bold text-rose-900">{data.summary.class}</p>
            </ModuleCard>
          </div>

          {data.summary.total === 0 ? (
            <ModuleCard bodyClassName="p-5">
              <p className="text-sm text-emerald-800 font-semibold">
                Aucun conflit inter-profs détecté pour la semaine {data.weekType}.
              </p>
            </ModuleCard>
          ) : (
            <ModuleCard bodyClassName="p-0 overflow-hidden">
              <div className="border-b border-slate-100 px-5 py-3">
                <h2 className="text-sm font-bold text-slate-800">
                  {data.summary.total} conflit{data.summary.total !== 1 ? "s" : ""} détecté
                  {data.summary.total !== 1 ? "s" : ""}
                </h2>
                <p className="text-xs text-slate-500 mt-0.5">
                  Chevauchements salle ou classe entre deux enseignants.
                </p>
              </div>
              <ul className="divide-y divide-slate-100 max-h-[420px] overflow-y-auto">
                {data.conflicts.map((c, i) => (
                  <li key={`${c.kind}-${c.start}-${i}`} className="px-5 py-3 text-sm">
                    <span
                      className={`inline-block rounded-full px-2 py-0.5 text-[10px] font-black uppercase mr-2 ${
                        c.kind === "room"
                          ? "bg-amber-100 text-amber-800"
                          : "bg-rose-100 text-rose-800"
                      }`}
                    >
                      {c.kind === "room" ? "Salle" : "Classe"}
                    </span>
                    <span className="text-slate-800">{c.message}</span>
                    {c.classes.length > 0 ? (
                      <div className="mt-1 flex flex-wrap gap-2">
                        {c.classes.map((cl) => (
                          <Link
                            key={cl}
                            href={`/edt-classe?classe=${encodeURIComponent(cl)}`}
                            className="text-xs font-bold text-indigo-600 hover:underline"
                          >
                            EDT {cl}
                          </Link>
                        ))}
                      </div>
                    ) : null}
                  </li>
                ))}
              </ul>
            </ModuleCard>
          )}

          <ModuleCard bodyClassName="p-5">
            <h2 className="text-sm font-bold text-slate-800 mb-3">Classes avec EDT</h2>
            <div className="flex flex-wrap gap-2">
              {data.classes.map((cl) => (
                <Link
                  key={cl}
                  href={`/edt-classe?classe=${encodeURIComponent(cl)}`}
                  className="rounded-full border border-slate-200 bg-white px-3 py-1 text-xs font-semibold text-slate-700 hover:border-indigo-300 hover:text-indigo-700"
                >
                  {cl}
                </Link>
              ))}
            </div>
          </ModuleCard>
        </>
      ) : null}
    </div>
  );
}
