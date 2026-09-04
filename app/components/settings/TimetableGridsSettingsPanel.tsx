"use client";

import { useCallback, useEffect, useState } from "react";
import {
  defaultTimetableGridsConfig,
  emptyTimetableGrid,
  emptyTimetablePeriod,
  parseTimetableGridsConfig,
  type TimetableGrid,
  type TimetableGridsConfig,
  type TimetablePeriod,
  type TimetablePeriodKind,
} from "@/app/lib/rh/timetable-grids";

const KIND_LABELS: Record<TimetablePeriodKind, string> = {
  lesson: "Cours",
  break: "Pause",
  lunch: "Midi",
};

export default function TimetableGridsSettingsPanel({
  establishments = [],
}: {
  establishments?: Array<{ id: string; label: string; kind?: string }>;
}) {
  const [cfg, setCfg] = useState<TimetableGridsConfig>(defaultTimetableGridsConfig());
  const [activeGridId, setActiveGridId] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/settings", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      const parsed = parseTimetableGridsConfig(j.config?.timetableGrids);
      setCfg(parsed);
      setActiveGridId(parsed.defaultGridId || parsed.grids[0]?.id || "");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur de chargement");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const activeGrid = cfg.grids.find((g) => g.id === activeGridId) || cfg.grids[0] || null;

  const updateGrid = (next: TimetableGrid) => {
    setCfg((prev) => ({
      ...prev,
      grids: prev.grids.map((g) => (g.id === next.id ? next : g)),
    }));
  };

  const updatePeriod = (periodId: string, patch: Partial<TimetablePeriod>) => {
    if (!activeGrid) return;
    updateGrid({
      ...activeGrid,
      periods: activeGrid.periods.map((p) =>
        p.id === periodId ? { ...p, ...patch } : p,
      ),
    });
  };

  const save = async () => {
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      const body = parseTimetableGridsConfig(cfg);
      const res = await fetch("/api/settings/timetable-grids", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      const saved = parseTimetableGridsConfig(j.config?.timetableGrids ?? body);
      setCfg(saved);
      setMessage("Grilles horaires enregistrées.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur d’enregistrement");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement des grilles horaires…</p>;
  }

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-lg font-black text-slate-900">Grilles horaires</h2>
        <p className="text-sm text-slate-600 mt-1 max-w-2xl">
          Sonneries et périodes de cours par cycle (école, collège, lycée). Utilisées pour
          l’édition rapide des EDT (début + nombre d’heures, pauses respectées) et, plus
          tard, pour les réservations de salles.
        </p>
      </div>

      {error ? (
        <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">
          {error}
        </p>
      ) : null}
      {message ? (
        <p className="text-sm text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2">
          {message}
        </p>
      ) : null}

      <div className="flex flex-wrap gap-2 items-center">
        {cfg.grids.map((g) => (
          <button
            key={g.id}
            type="button"
            onClick={() => setActiveGridId(g.id)}
            className={`px-3 py-1.5 rounded-xl text-xs font-bold border ${
              activeGrid?.id === g.id
                ? "bg-slate-900 text-white border-slate-900"
                : "bg-white text-slate-700 border-slate-200 hover:bg-slate-50"
            }`}
          >
            {g.label}
            {cfg.defaultGridId === g.id ? " ★" : ""}
          </button>
        ))}
        <button
          type="button"
          className="px-3 py-1.5 rounded-xl text-xs font-bold border border-dashed border-indigo-300 text-indigo-700 hover:bg-indigo-50"
          onClick={() => {
            const g = emptyTimetableGrid({
              label: `Grille ${cfg.grids.length + 1}`,
            });
            setCfg((prev) => ({
              ...prev,
              grids: [...prev.grids, g],
              defaultGridId: prev.defaultGridId || g.id,
            }));
            setActiveGridId(g.id);
          }}
        >
          + Ajouter une grille
        </button>
      </div>

      {activeGrid ? (
        <div className="rounded-2xl border border-slate-200 bg-white p-4 space-y-4">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-3">
            <label className="text-xs font-bold text-slate-600 space-y-1 block">
              Libellé
              <input
                className="w-full border rounded-lg px-2 py-1.5 text-sm"
                value={activeGrid.label}
                onChange={(e) => updateGrid({ ...activeGrid, label: e.target.value })}
              />
            </label>
            <label className="text-xs font-bold text-slate-600 space-y-1 block">
              Cycle
              <select
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={activeGrid.kind || "custom"}
                onChange={(e) =>
                  updateGrid({
                    ...activeGrid,
                    kind: e.target.value as TimetableGrid["kind"],
                  })
                }
              >
                <option value="ecole">École</option>
                <option value="college">Collège</option>
                <option value="lycee">Lycée</option>
                <option value="custom">Autre</option>
              </select>
            </label>
            <label className="text-xs font-bold text-slate-600 space-y-1 block">
              Site lié (optionnel)
              <select
                className="w-full border rounded-lg px-2 py-1.5 text-sm bg-white"
                value={activeGrid.establishmentId || ""}
                onChange={(e) =>
                  updateGrid({
                    ...activeGrid,
                    establishmentId: e.target.value || null,
                  })
                }
              >
                <option value="">— Aucun —</option>
                {establishments.map((e) => (
                  <option key={e.id} value={e.id}>
                    {e.label}
                  </option>
                ))}
              </select>
            </label>
            <div className="flex items-end gap-2">
              <button
                type="button"
                className="px-3 py-1.5 rounded-xl text-xs font-bold border border-slate-200 text-slate-700 hover:bg-slate-50"
                onClick={() =>
                  setCfg((prev) => ({ ...prev, defaultGridId: activeGrid.id }))
                }
              >
                Définir par défaut
              </button>
              {cfg.grids.length > 1 ? (
                <button
                  type="button"
                  className="px-3 py-1.5 rounded-xl text-xs font-bold border border-rose-200 text-rose-700 hover:bg-rose-50"
                  onClick={() => {
                    setCfg((prev) => {
                      const grids = prev.grids.filter((g) => g.id !== activeGrid.id);
                      const defaultGridId =
                        prev.defaultGridId === activeGrid.id
                          ? grids[0]?.id
                          : prev.defaultGridId;
                      return { grids, defaultGridId };
                    });
                    setActiveGridId((id) =>
                      id === activeGrid.id ? cfg.grids.find((g) => g.id !== id)?.id || "" : id,
                    );
                  }}
                >
                  Supprimer
                </button>
              ) : null}
            </div>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-[11px] uppercase tracking-wide text-slate-500 border-b">
                  <th className="py-2 pr-2">Type</th>
                  <th className="py-2 pr-2">Libellé</th>
                  <th className="py-2 pr-2">Début</th>
                  <th className="py-2 pr-2">Fin</th>
                  <th className="py-2" />
                </tr>
              </thead>
              <tbody>
                {activeGrid.periods.map((p) => (
                  <tr key={p.id} className="border-b border-slate-100">
                    <td className="py-1.5 pr-2">
                      <select
                        className="border rounded-lg px-2 py-1 text-xs bg-white"
                        value={p.kind}
                        onChange={(e) =>
                          updatePeriod(p.id, {
                            kind: e.target.value as TimetablePeriodKind,
                          })
                        }
                      >
                        {(Object.keys(KIND_LABELS) as TimetablePeriodKind[]).map((k) => (
                          <option key={k} value={k}>
                            {KIND_LABELS[k]}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="py-1.5 pr-2">
                      <input
                        className="w-full min-w-[8rem] border rounded-lg px-2 py-1 text-xs"
                        value={p.label || ""}
                        onChange={(e) => updatePeriod(p.id, { label: e.target.value })}
                      />
                    </td>
                    <td className="py-1.5 pr-2">
                      <input
                        type="time"
                        className="border rounded-lg px-2 py-1 text-xs"
                        value={p.start}
                        onChange={(e) => updatePeriod(p.id, { start: e.target.value })}
                      />
                    </td>
                    <td className="py-1.5 pr-2">
                      <input
                        type="time"
                        className="border rounded-lg px-2 py-1 text-xs"
                        value={p.end}
                        onChange={(e) => updatePeriod(p.id, { end: e.target.value })}
                      />
                    </td>
                    <td className="py-1.5">
                      <button
                        type="button"
                        className="text-xs font-bold text-rose-600 hover:underline"
                        onClick={() =>
                          updateGrid({
                            ...activeGrid,
                            periods: activeGrid.periods.filter((x) => x.id !== p.id),
                          })
                        }
                      >
                        Retirer
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              className="px-3 py-1.5 rounded-xl text-xs font-bold border border-slate-200 hover:bg-slate-50"
              onClick={() =>
                updateGrid({
                  ...activeGrid,
                  periods: [...activeGrid.periods, emptyTimetablePeriod("lesson")],
                })
              }
            >
              + Heure de cours
            </button>
            <button
              type="button"
              className="px-3 py-1.5 rounded-xl text-xs font-bold border border-slate-200 hover:bg-slate-50"
              onClick={() =>
                updateGrid({
                  ...activeGrid,
                  periods: [
                    ...activeGrid.periods,
                    emptyTimetablePeriod("break", "10:20", "10:35"),
                  ],
                })
              }
            >
              + Pause
            </button>
            <button
              type="button"
              className="px-3 py-1.5 rounded-xl text-xs font-bold border border-slate-200 hover:bg-slate-50"
              onClick={() =>
                updateGrid({
                  ...activeGrid,
                  periods: [
                    ...activeGrid.periods,
                    emptyTimetablePeriod("lunch", "12:25", "13:30"),
                  ],
                })
              }
            >
              + Pause midi
            </button>
          </div>
        </div>
      ) : null}

      <div className="flex justify-end">
        <button
          type="button"
          disabled={saving}
          onClick={() => void save()}
          className="px-4 py-2 rounded-xl text-sm font-bold bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-60"
        >
          {saving ? "Enregistrement…" : "Enregistrer les grilles"}
        </button>
      </div>
    </div>
  );
}
