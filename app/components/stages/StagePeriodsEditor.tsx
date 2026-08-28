"use client";

import { useCallback, useEffect, useState } from "react";
import type {
  StageClassPeriod,
  StageClassStageConfig,
  StagePeriodReminder,
} from "@/app/lib/stage-periods-config";

function currentSchoolYearLabel() {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 8) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}

function uid(prefix: string) {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
}

function emptyClassConfig(className: string): StageClassStageConfig {
  return { className, enabled: false, periods: [], reminders: [] };
}

export default function StagePeriodsEditor({
  initialYear,
  onSaved,
}: {
  initialYear?: string;
  onSaved?: (message: string) => void;
}) {
  const [schoolYear, setSchoolYear] = useState(initialYear || currentSchoolYearLabel());
  const [planningClasses, setPlanningClasses] = useState<string[]>([]);
  const [classes, setClasses] = useState<StageClassStageConfig[]>([]);
  const [expandedClass, setExpandedClass] = useState<string | null>(null);
  const [updatedAt, setUpdatedAt] = useState<string | null>(null);
  const [updatedBy, setUpdatedBy] = useState<string | null>(null);
  const [previousConfig, setPreviousConfig] = useState<{ schoolYear: string; classes: StageClassStageConfig[] } | null>(
    null,
  );
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [customClass, setCustomClass] = useState("");

  const load = useCallback(async (year: string) => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/stages/periods?schoolYear=${encodeURIComponent(year)}`, {
        cache: "no-store",
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur chargement périodes");

      const planning: string[] = data.planningClasses || [];
      setPlanningClasses(planning);
      setPreviousConfig(data.previousConfig || null);
      setUpdatedAt(data.config?.updatedAt || null);
      setUpdatedBy(data.config?.updatedBy || null);

      const byName = new Map<string, StageClassStageConfig>();
      for (const name of planning) {
        byName.set(name, emptyClassConfig(name));
      }
      for (const c of (data.config?.classes || []) as StageClassStageConfig[]) {
        byName.set(c.className, c);
      }
      setClasses([...byName.values()].sort((a, b) =>
        a.className.localeCompare(b.className, "fr", { sensitivity: "base" }),
      ));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load(schoolYear);
  }, [schoolYear, load]);

  function updateClass(className: string, patch: Partial<StageClassStageConfig>) {
    setClasses((prev) =>
      prev.map((c) => (c.className === className ? { ...c, ...patch } : c)),
    );
  }

  function addPeriod(className: string) {
    setClasses((prev) =>
      prev.map((c) => {
        if (c.className !== className) return c;
        const period: StageClassPeriod = {
          id: uid("per"),
          label: `Période ${c.periods.length + 1}`,
          periodStart: "",
          periodEnd: "",
        };
        return { ...c, periods: [...c.periods, period] };
      }),
    );
  }

  function updatePeriod(className: string, periodId: string, patch: Partial<StageClassPeriod>) {
    setClasses((prev) =>
      prev.map((c) => {
        if (c.className !== className) return c;
        return {
          ...c,
          periods: c.periods.map((p) => (p.id === periodId ? { ...p, ...patch } : p)),
        };
      }),
    );
  }

  function removePeriod(className: string, periodId: string) {
    setClasses((prev) =>
      prev.map((c) =>
        c.className === className
          ? { ...c, periods: c.periods.filter((p) => p.id !== periodId) }
          : c,
      ),
    );
  }

  function addReminder(className: string) {
    setClasses((prev) =>
      prev.map((c) => {
        if (c.className !== className) return c;
        const reminder: StagePeriodReminder = {
          id: uid("rem"),
          label: "Rappel dates",
          message: "",
        };
        return { ...c, reminders: [...c.reminders, reminder] };
      }),
    );
  }

  function updateReminder(
    className: string,
    reminderId: string,
    patch: Partial<StagePeriodReminder>,
  ) {
    setClasses((prev) =>
      prev.map((c) => {
        if (c.className !== className) return c;
        return {
          ...c,
          reminders: c.reminders.map((r) => (r.id === reminderId ? { ...r, ...patch } : r)),
        };
      }),
    );
  }

  function removeReminder(className: string, reminderId: string) {
    setClasses((prev) =>
      prev.map((c) =>
        c.className === className
          ? { ...c, reminders: c.reminders.filter((r) => r.id !== reminderId) }
          : c,
      ),
    );
  }

  function copyFromPreviousYear() {
    if (!previousConfig?.classes.length) return;
    const byName = new Map(previousConfig.classes.map((c) => [c.className, c]));
    setClasses((prev) =>
      prev.map((c) => byName.get(c.className) ?? c),
    );
    onSaved?.(`Configuration copiée depuis ${previousConfig.schoolYear}.`);
  }

  function addCustomClass() {
    const name = customClass.trim();
    if (!name || planningClasses.includes(name)) {
      setCustomClass("");
      return;
    }
    setPlanningClasses((prev) =>
      [...prev, name].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" })),
    );
    setClasses((prev) =>
      [...prev, emptyClassConfig(name)].sort((a, b) =>
        a.className.localeCompare(b.className, "fr", { sensitivity: "base" }),
      ),
    );
    setCustomClass("");
  }

  async function save() {
    setBusy(true);
    setError(null);
    try {
      const payload = classes.filter((c) => c.enabled || c.periods.length || c.reminders.length);
      const res = await fetch("/api/stages/periods", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ schoolYear, classes: payload }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setUpdatedAt(data.config?.updatedAt || null);
      setUpdatedBy(data.config?.updatedBy || null);
      const enabledCount = payload.filter((c) => c.enabled).length;
      onSaved?.(
        `Périodes de stage enregistrées pour ${schoolYear} (${enabledCount} classe(s) activée(s)).`,
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  const enabledCount = classes.filter((c) => c.enabled).length;

  if (loading) {
    return <p className="text-sm text-stone-500">Chargement des périodes…</p>;
  }

  return (
    <div className="space-y-4">
      {error && (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">{error}</p>
      )}

      <div className="flex flex-wrap items-end gap-3">
        <label className="block text-sm">
          Année scolaire
          <input
            className="mt-1 block w-40 rounded-lg border border-stone-300 px-3 py-2"
            value={schoolYear}
            onChange={(e) => setSchoolYear(e.target.value)}
            placeholder="2025-2026"
          />
        </label>
        {previousConfig && previousConfig.classes.length > 0 && (
          <button
            type="button"
            onClick={copyFromPreviousYear}
            className="rounded-lg border border-stone-300 px-3 py-2 text-sm font-semibold text-stone-700 hover:bg-stone-50"
          >
            Reprendre {previousConfig.schoolYear}
          </button>
        )}
      </div>

      <p className="text-sm text-stone-600 max-w-3xl">
        Activez uniquement les classes concernées par les stages. Configurez les périodes officielles et
        les rappels affichés sur le formulaire public (ex. « Attention : dates des 2de… »). Seules les
        classes activées apparaissent dans les référents et le formulaire élève.
      </p>

      {updatedAt && (
        <p className="text-xs text-stone-500">
          Dernière mise à jour : {new Date(updatedAt).toLocaleString("fr-FR")}
          {updatedBy ? ` par ${updatedBy}` : ""} — {enabledCount} classe(s) activée(s)
        </p>
      )}

      <div className="space-y-2 max-h-[520px] overflow-y-auto rounded-xl border border-stone-200">
        {classes.map((c) => (
          <div key={c.className} className="border-b border-stone-100 last:border-0 bg-white">
            <div className="flex flex-wrap items-center gap-3 px-4 py-3">
              <label className="flex items-center gap-2 shrink-0">
                <input
                  type="checkbox"
                  checked={c.enabled}
                  onChange={(e) => updateClass(c.className, { enabled: e.target.checked })}
                />
                <span className="font-bold text-[#1F3D2B] w-16">{c.className}</span>
              </label>
              <button
                type="button"
                className="text-xs font-semibold text-[#2F6B4A] underline"
                onClick={() =>
                  setExpandedClass(expandedClass === c.className ? null : c.className)
                }
              >
                {expandedClass === c.className ? "Masquer" : "Périodes & rappels"}
              </button>
              {c.enabled && (
                <span className="text-xs text-emerald-700">
                  {c.periods.length} période(s) · {c.reminders.length} rappel(s)
                </span>
              )}
            </div>

            {expandedClass === c.className && (
              <div className="px-4 pb-4 space-y-4 bg-stone-50/60">
                <div>
                  <div className="flex items-center justify-between">
                    <h4 className="text-xs font-bold text-stone-700">Périodes de stage</h4>
                    <button
                      type="button"
                      onClick={() => addPeriod(c.className)}
                      className="text-xs text-[#2F6B4A] font-semibold underline"
                    >
                      + Ajouter une période
                    </button>
                  </div>
                  {c.periods.length === 0 ? (
                    <p className="mt-2 text-xs text-stone-500">Aucune période définie.</p>
                  ) : (
                    <ul className="mt-2 space-y-2">
                      {c.periods.map((p) => (
                        <li key={p.id} className="rounded-lg border border-stone-200 bg-white p-3 space-y-2">
                          <input
                            className="w-full rounded border px-2 py-1 text-xs"
                            placeholder="Libellé (ex. PFMP 1)"
                            value={p.label}
                            onChange={(e) =>
                              updatePeriod(c.className, p.id, { label: e.target.value })
                            }
                          />
                          <div className="grid grid-cols-2 gap-2">
                            <input
                              type="date"
                              className="rounded border px-2 py-1 text-xs"
                              value={p.periodStart}
                              onChange={(e) =>
                                updatePeriod(c.className, p.id, { periodStart: e.target.value })
                              }
                            />
                            <input
                              type="date"
                              className="rounded border px-2 py-1 text-xs"
                              value={p.periodEnd}
                              onChange={(e) =>
                                updatePeriod(c.className, p.id, { periodEnd: e.target.value })
                              }
                            />
                          </div>
                          <button
                            type="button"
                            onClick={() => removePeriod(c.className, p.id)}
                            className="text-xs text-rose-700 underline"
                          >
                            Supprimer
                          </button>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>

                <div>
                  <div className="flex items-center justify-between">
                    <h4 className="text-xs font-bold text-stone-700">Rappels affichés aux familles</h4>
                    <button
                      type="button"
                      onClick={() => addReminder(c.className)}
                      className="text-xs text-[#2F6B4A] font-semibold underline"
                    >
                      + Ajouter un rappel
                    </button>
                  </div>
                  {c.reminders.length === 0 ? (
                    <p className="mt-2 text-xs text-stone-500">Aucun rappel — ajoutez un message d&apos;attention sur les dates.</p>
                  ) : (
                    <ul className="mt-2 space-y-2">
                      {c.reminders.map((r) => (
                        <li key={r.id} className="rounded-lg border border-amber-200 bg-amber-50/50 p-3 space-y-2">
                          <input
                            className="w-full rounded border px-2 py-1 text-xs"
                            placeholder="Titre du rappel"
                            value={r.label}
                            onChange={(e) =>
                              updateReminder(c.className, r.id, { label: e.target.value })
                            }
                          />
                          <textarea
                            className="w-full rounded border px-2 py-1 text-xs min-h-[60px]"
                            placeholder="Message (ex. Votre stage doit se situer entre le…)"
                            value={r.message}
                            onChange={(e) =>
                              updateReminder(c.className, r.id, { message: e.target.value })
                            }
                          />
                          <div className="grid grid-cols-2 gap-2">
                            <input
                              type="date"
                              className="rounded border px-2 py-1 text-xs"
                              value={r.periodStart || ""}
                              onChange={(e) =>
                                updateReminder(c.className, r.id, {
                                  periodStart: e.target.value || undefined,
                                })
                              }
                            />
                            <input
                              type="date"
                              className="rounded border px-2 py-1 text-xs"
                              value={r.periodEnd || ""}
                              onChange={(e) =>
                                updateReminder(c.className, r.id, {
                                  periodEnd: e.target.value || undefined,
                                })
                              }
                            />
                          </div>
                          <button
                            type="button"
                            onClick={() => removeReminder(c.className, r.id)}
                            className="text-xs text-rose-700 underline"
                          >
                            Supprimer
                          </button>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <input
          className="rounded-lg border border-stone-300 px-3 py-2 text-sm"
          placeholder="Classe hors liste"
          value={customClass}
          onChange={(e) => setCustomClass(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addCustomClass();
            }
          }}
        />
        <button
          type="button"
          onClick={addCustomClass}
          className="rounded-lg border border-stone-300 px-3 py-2 text-sm font-semibold text-stone-700"
        >
          Ajouter une classe
        </button>
      </div>

      <button
        type="button"
        disabled={busy}
        onClick={() => void save()}
        className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
      >
        {busy ? "Enregistrement…" : "Enregistrer les périodes de stage"}
      </button>
    </div>
  );
}
