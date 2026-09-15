"use client";

import { useCallback, useEffect, useState } from "react";
import {
  defaultCollegeConstraints,
  defaultLyceeConstraints,
  type StageBlockedPeriod,
  type StageConstraintsConfig,
  type StageCycleConstraints,
  type StageCycleKind,
} from "@/app/lib/stage-constraints";
import { STAGE_WEEKDAY_LABELS, STAGE_WEEKDAYS } from "@/app/lib/stage-schedule";
import type { StageWeekday } from "@/app/lib/stage-types";

const CYCLES: Array<{ id: StageCycleKind; label: string; hint: string }> = [
  {
    id: "college",
    label: "Collège",
    hint: "Par défaut : lun–ven, 6h–20h, pas de week-end.",
  },
  {
    id: "lycee",
    label: "Lycée",
    hint: "Par défaut : lun–sam, 6h–22h (20h si moins de 16 ans).",
  },
  {
    id: "ecole",
    label: "École",
    hint: "Aligné sur le collège par défaut (rare pour les stages).",
  },
];

function uid(prefix: string) {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
}

function emptyBlocked(): StageBlockedPeriod {
  return {
    id: uid("blk"),
    label: "Période fermée",
    periodStart: "",
    periodEnd: "",
  };
}

function CycleRulesEditor({
  label,
  hint,
  value,
  onChange,
}: {
  label: string;
  hint: string;
  value: StageCycleConstraints;
  onChange: (next: StageCycleConstraints) => void;
}) {
  const allowed = new Set(value.allowedWeekdays);

  function toggleWeekday(weekday: StageWeekday, enabled: boolean) {
    const next = enabled
      ? [...new Set([...value.allowedWeekdays, weekday])].sort((a, b) => a - b)
      : value.allowedWeekdays.filter((w) => w !== weekday);
    onChange({ ...value, allowedWeekdays: next as StageWeekday[] });
  }

  return (
    <div className="space-y-4 rounded-xl border border-stone-200 bg-stone-50/80 p-4">
      <div>
        <h3 className="text-sm font-bold text-[#1F3D2B]">{label}</h3>
        <p className="mt-1 text-xs text-stone-500">{hint}</p>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block text-xs font-semibold text-stone-600">
          Heure la plus tôt
          <input
            type="time"
            className="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm"
            value={value.earliestStart}
            onChange={(e) => onChange({ ...value, earliestStart: e.target.value })}
          />
        </label>
        <label className="block text-xs font-semibold text-stone-600">
          Heure la plus tard
          <input
            type="time"
            className="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm"
            value={value.latestEnd}
            onChange={(e) => onChange({ ...value, latestEnd: e.target.value })}
          />
        </label>
        <label className="block text-xs font-semibold text-stone-600 sm:col-span-2">
          Heure max si moins de 16 ans (optionnel)
          <input
            type="time"
            className="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm"
            value={value.latestEndUnder16 || ""}
            onChange={(e) =>
              onChange({
                ...value,
                latestEndUnder16: e.target.value.trim() || undefined,
              })
            }
          />
          <span className="mt-1 block text-[11px] font-normal text-stone-500">
            Ex. lycée jusqu&apos;à 22h, mais 20h pour les moins de 16 ans. Laissez vide pour
            appliquer la même borne à tous.
          </span>
        </label>
      </div>

      <div>
        <p className="text-xs font-bold text-[#1F3D2B]">Jours autorisés</p>
        <p className="mt-1 text-[11px] text-stone-500">
          Le dimanche reste toujours exclu. Décochez le samedi pour le collège, par exemple.
        </p>
        <div className="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-3">
          {STAGE_WEEKDAYS.map((weekday) => {
            const checked = allowed.has(weekday);
            return (
              <label
                key={weekday}
                className={`flex cursor-pointer items-center gap-2 rounded-lg border px-3 py-2 text-sm font-semibold ${
                  checked
                    ? "border-[#2F6B4A] bg-[#e8f5ee] text-[#1F3D2B]"
                    : "border-stone-200 bg-white text-stone-500"
                }`}
              >
                <input
                  type="checkbox"
                  className="accent-[#2F6B4A]"
                  checked={checked}
                  onChange={(e) => toggleWeekday(weekday, e.target.checked)}
                />
                {STAGE_WEEKDAY_LABELS[weekday]}
              </label>
            );
          })}
        </div>
      </div>

      <div className="grid gap-3 sm:grid-cols-3">
        <label className="block text-xs font-semibold text-stone-600">
          Max h/semaine (&lt; 15 ans)
          <input
            type="number"
            min={1}
            max={48}
            step={0.5}
            className="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm"
            value={value.maxWeeklyHoursUnder15}
            onChange={(e) =>
              onChange({
                ...value,
                maxWeeklyHoursUnder15: Number(e.target.value) || value.maxWeeklyHoursUnder15,
              })
            }
          />
        </label>
        <label className="block text-xs font-semibold text-stone-600">
          Max h/semaine (≥ 15 ans)
          <input
            type="number"
            min={1}
            max={48}
            step={0.5}
            className="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm"
            value={value.maxWeeklyHoursFrom15}
            onChange={(e) =>
              onChange({
                ...value,
                maxWeeklyHoursFrom15: Number(e.target.value) || value.maxWeeklyHoursFrom15,
              })
            }
          />
        </label>
        <label className="block text-xs font-semibold text-stone-600">
          Max h/jour
          <input
            type="number"
            min={1}
            max={12}
            step={0.5}
            className="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm"
            value={value.maxDailyHours}
            onChange={(e) =>
              onChange({
                ...value,
                maxDailyHours: Number(e.target.value) || value.maxDailyHours,
              })
            }
          />
        </label>
      </div>
    </div>
  );
}

export default function StageConstraintsEditor({
  onSaved,
}: {
  onSaved?: (message: string) => void;
}) {
  const [schoolYear, setSchoolYear] = useState("");
  const [byCycle, setByCycle] = useState<StageConstraintsConfig["byCycle"]>(() => ({
    college: defaultCollegeConstraints(),
    lycee: defaultLyceeConstraints(),
    ecole: defaultCollegeConstraints(),
  }));
  const [blockedPeriods, setBlockedPeriods] = useState<StageBlockedPeriod[]>([]);
  const [updatedAt, setUpdatedAt] = useState<string | null>(null);
  const [updatedBy, setUpdatedBy] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeCycle, setActiveCycle] = useState<StageCycleKind>("college");

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/constraints", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur chargement contraintes");
      const config = data.config as StageConstraintsConfig;
      setSchoolYear(config.schoolYear || data.schoolYear || "");
      setByCycle(config.byCycle);
      setBlockedPeriods(Array.isArray(config.blockedPeriods) ? config.blockedPeriods : []);
      setUpdatedAt(config.updatedAt || null);
      setUpdatedBy(config.updatedBy || null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function save() {
    setBusy(true);
    setError(null);
    try {
      for (const blocked of blockedPeriods) {
        if (!blocked.label.trim() || !blocked.periodStart || !blocked.periodEnd) {
          throw new Error("Chaque période bloquée doit avoir un libellé, une date de début et une date de fin.");
        }
        if (blocked.periodEnd < blocked.periodStart) {
          throw new Error(`La période « ${blocked.label} » a une date de fin avant le début.`);
        }
      }
      for (const cycle of CYCLES) {
        if (byCycle[cycle.id].allowedWeekdays.length === 0) {
          throw new Error(`${cycle.label} : sélectionnez au moins un jour autorisé.`);
        }
      }

      const res = await fetch("/api/stages/constraints", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ schoolYear, byCycle, blockedPeriods }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Enregistrement impossible");
      const config = data.config as StageConstraintsConfig;
      setByCycle(config.byCycle);
      setBlockedPeriods(config.blockedPeriods);
      setUpdatedAt(config.updatedAt);
      setUpdatedBy(config.updatedBy || null);
      onSaved?.("Contraintes horaires stages enregistrées.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  function resetCycleDefaults() {
    setByCycle({
      college: defaultCollegeConstraints(),
      lycee: defaultLyceeConstraints(),
      ecole: defaultCollegeConstraints(),
    });
  }

  if (loading) {
    return <p className="text-sm text-stone-500">Chargement des contraintes…</p>;
  }

  return (
    <div className="space-y-6">
      <p className="text-sm text-stone-600 max-w-3xl leading-relaxed">
        Ces règles bridgent la saisie des horaires (préconvention élève et demandes tuteur) :
        plage horaire, jours de la semaine, plafond d&apos;heures selon l&apos;âge, et périodes
        entièrement fermées (ex. 2ᵉ semaine de vacances).
      </p>

      {(updatedAt || updatedBy) && (
        <p className="text-xs text-stone-500">
          Dernière mise à jour
          {updatedAt
            ? ` le ${new Date(updatedAt).toLocaleString("fr-FR")}`
            : ""}
          {updatedBy ? ` par ${updatedBy}` : ""}.
        </p>
      )}

      {error ? (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
          {error}
        </p>
      ) : null}

      <div className="flex flex-wrap gap-2">
        {CYCLES.map((cycle) => (
          <button
            key={cycle.id}
            type="button"
            onClick={() => setActiveCycle(cycle.id)}
            className={`rounded-lg px-3 py-1.5 text-sm font-semibold transition ${
              activeCycle === cycle.id
                ? "bg-[#2F6B4A] text-white"
                : "bg-stone-100 text-stone-700 hover:bg-stone-200"
            }`}
          >
            {cycle.label}
          </button>
        ))}
        <button
          type="button"
          onClick={resetCycleDefaults}
          className="ml-auto rounded-lg border border-stone-300 bg-white px-3 py-1.5 text-xs font-semibold text-stone-700 hover:bg-stone-50"
        >
          Réinitialiser les défauts légaux
        </button>
      </div>

      {CYCLES.filter((c) => c.id === activeCycle).map((cycle) => (
        <CycleRulesEditor
          key={cycle.id}
          label={cycle.label}
          hint={cycle.hint}
          value={byCycle[cycle.id]}
          onChange={(next) => setByCycle((prev) => ({ ...prev, [cycle.id]: next }))}
        />
      ))}

      <section className="space-y-3 rounded-xl border border-rose-200 bg-rose-50/40 p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold text-rose-950">Périodes bloquées</h3>
            <p className="mt-1 text-xs text-rose-900/80 max-w-2xl leading-relaxed">
              Aucune présence en stage n&apos;est acceptée sur ces dates (ex. établissement fermé
              la 2ᵉ semaine des vacances). La 1ʳᵉ semaine ouverte n&apos;a pas besoin d&apos;être
              listée ici.
            </p>
          </div>
          <button
            type="button"
            onClick={() => setBlockedPeriods((prev) => [...prev, emptyBlocked()])}
            className="rounded-lg bg-rose-800 px-3 py-1.5 text-xs font-semibold text-white hover:bg-rose-900"
          >
            Ajouter une période
          </button>
        </div>

        {blockedPeriods.length === 0 ? (
          <p className="text-xs text-rose-800/80">Aucune période bloquée pour l&apos;instant.</p>
        ) : (
          <ul className="space-y-3">
            {blockedPeriods.map((blocked) => (
              <li
                key={blocked.id}
                className="grid gap-3 rounded-lg border border-rose-200 bg-white p-3 sm:grid-cols-[1fr_auto_auto_auto]"
              >
                <label className="block text-xs font-semibold text-stone-600">
                  Libellé
                  <input
                    type="text"
                    className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2 text-sm"
                    value={blocked.label}
                    onChange={(e) =>
                      setBlockedPeriods((prev) =>
                        prev.map((p) =>
                          p.id === blocked.id ? { ...p, label: e.target.value } : p,
                        ),
                      )
                    }
                    placeholder="Ex. Vacances février — semaine 2"
                  />
                </label>
                <label className="block text-xs font-semibold text-stone-600">
                  Du
                  <input
                    type="date"
                    className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2 text-sm"
                    value={blocked.periodStart}
                    onChange={(e) =>
                      setBlockedPeriods((prev) =>
                        prev.map((p) =>
                          p.id === blocked.id ? { ...p, periodStart: e.target.value } : p,
                        ),
                      )
                    }
                  />
                </label>
                <label className="block text-xs font-semibold text-stone-600">
                  Au
                  <input
                    type="date"
                    className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2 text-sm"
                    value={blocked.periodEnd}
                    onChange={(e) =>
                      setBlockedPeriods((prev) =>
                        prev.map((p) =>
                          p.id === blocked.id ? { ...p, periodEnd: e.target.value } : p,
                        ),
                      )
                    }
                  />
                </label>
                <div className="flex items-end">
                  <button
                    type="button"
                    onClick={() =>
                      setBlockedPeriods((prev) => prev.filter((p) => p.id !== blocked.id))
                    }
                    className="rounded-lg border border-rose-300 px-3 py-2 text-xs font-semibold text-rose-800 hover:bg-rose-50"
                  >
                    Retirer
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>

      <div className="flex justify-end">
        <button
          type="button"
          disabled={busy}
          onClick={() => void save()}
          className="rounded-xl bg-[#2F6B4A] px-5 py-2.5 text-sm font-bold text-white shadow-sm hover:bg-[#265a3d] disabled:opacity-60"
        >
          {busy ? "Enregistrement…" : "Enregistrer les contraintes"}
        </button>
      </div>
    </div>
  );
}
