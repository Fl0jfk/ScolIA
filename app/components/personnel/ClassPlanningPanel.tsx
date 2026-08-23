"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { WeekGrid } from "@/app/components/personnel/RhPlanningEditors";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import { dash } from "@/app/lib/dashboard-brand";
import {
  PLANNING_WEEKDAY_LABELS,
  type PlanningWeekday,
} from "@/app/lib/rh/planning-types";

type ClassScheduleSlot = {
  id: string;
  day: PlanningWeekday;
  start: string;
  end: string;
  subject: string;
  room: string | null;
  teacherName: string;
  teacherId: string;
  kind: "cours" | "remplacement";
  weekType: "A" | "B" | null;
  replacementDate?: string;
};

type LivePayload = {
  activity: {
    subject: string;
    room: string | null;
    start: string;
    end: string;
    teacherName: string;
    kind: "cours" | "remplacement";
    weekType: "A" | "B" | null;
  } | null;
  reason: string;
  label?: string;
};

type ApiPayload = {
  classe: string;
  classes: string[];
  weekType: "A" | "B";
  weekStart: string;
  weekEnd: string;
  parityLabel: "A" | "B";
  slots: ClassScheduleSlot[];
  slotCount: number;
  replacementsThisWeek: number;
  live: LivePayload;
  profScoped: boolean;
};

type Props = {
  initialClasse?: string;
  compact?: boolean;
};

function formatWeekRange(weekStart: string, weekEnd: string) {
  const start = new Date(`${weekStart}T12:00:00`);
  const end = new Date(`${weekEnd}T12:00:00`);
  const opts: Intl.DateTimeFormatOptions = { day: "numeric", month: "short" };
  return `${start.toLocaleDateString("fr-FR", opts)} – ${end.toLocaleDateString("fr-FR", {
    ...opts,
    year: "numeric",
  })}`;
}

export default function ClassPlanningPanel({ initialClasse = "", compact = false }: Props) {
  const [classes, setClasses] = useState<string[]>([]);
  const [classe, setClasse] = useState(initialClasse);
  const [weekType, setWeekType] = useState<"A" | "B">("A");
  const [data, setData] = useState<ApiPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [profScoped, setProfScoped] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams();
      if (classe) params.set("classe", classe);
      params.set("week", weekType);
      const res = await fetch(`/api/edt/classe?${params}`, { cache: "no-store" });
      const j = (await res.json()) as ApiPayload & { error?: string };
      if (!res.ok) throw new Error(j.error || `Erreur ${res.status}`);
      setClasses(j.classes || []);
      setProfScoped(Boolean(j.profScoped));
      if (!classe && j.classes?.length === 1) {
        setClasse(j.classes[0]!);
        return;
      }
      if (classe) {
        setData(j);
      } else {
        setData(null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur chargement");
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [classe, weekType]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (initialClasse && initialClasse !== classe) {
      setClasse(initialClasse);
    }
  }, [initialClasse, classe]);

  const slots = data?.slots ?? [];
  const gridSlots = useMemo(
    () => slots.map((s) => ({ id: s.id, day: s.day, start: s.start, end: s.end })),
    [slots],
  );

  const slotById = useMemo(() => new Map(slots.map((s) => [s.id, s])), [slots]);

  return (
    <div className="space-y-4">
      {!compact ? (
        <p className={`text-sm ${dash.textMid}`}>
          Emploi du temps agrégé par classe — semaines types A/B et remplacements de la semaine.
          {profScoped ? " Vous ne voyez que vos classes." : ""}
        </p>
      ) : null}

      <div className="flex flex-wrap items-end gap-3">
        <label className="block text-sm min-w-[160px]">
          <span className="text-xs font-bold text-slate-500 uppercase tracking-wide">Classe</span>
          <select
            className="mt-1 w-full border border-slate-200 rounded-xl px-3 py-2 text-sm font-medium bg-white"
            value={classe}
            onChange={(e) => setClasse(e.target.value)}
          >
            <option value="">— Choisir —</option>
            {classes.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </label>

        {classe ? (
          <>
            {(
              [
                ["A", "Semaine type A"],
                ["B", "Semaine type B"],
              ] as const
            ).map(([id, label]) => (
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
                {label}
              </button>
            ))}
          </>
        ) : null}
      </div>

      {error ? (
        <p className="text-sm text-rose-600 rounded-xl bg-rose-50 px-3 py-2">{error}</p>
      ) : null}

      {loading ? (
        <p className={`text-sm ${dash.textMid}`}>Chargement de l’EDT…</p>
      ) : null}

      {data && classe ? (
        <>
          {data.live.activity ? (
            <ModuleCard bodyClassName="px-4 py-3 bg-indigo-50/80 border-indigo-100">
              <p className="text-[10px] font-black uppercase tracking-wide text-indigo-600">
                En ce moment — {classe}
              </p>
              <p className="mt-1 text-lg font-bold text-indigo-950">{data.live.activity.subject}</p>
              <p className="text-sm text-indigo-900/90">
                {data.live.activity.start}–{data.live.activity.end}
                {data.live.activity.room ? ` · ${data.live.activity.room}` : ""}
                {" · "}
                {data.live.activity.teacherName}
              </p>
            </ModuleCard>
          ) : null}

          <div className="flex flex-wrap items-center gap-2 text-xs text-slate-500">
            <span>
              {formatWeekRange(data.weekStart, data.weekEnd)}
              {data.parityLabel ? ` · parité ${data.parityLabel} cette semaine` : ""}
            </span>
            <span className="font-semibold text-slate-700">
              {data.slotCount} créneau{data.slotCount !== 1 ? "x" : ""}
            </span>
            {data.replacementsThisWeek > 0 ? (
              <span className="text-violet-700 font-semibold">
                {data.replacementsThisWeek} remplacement
                {data.replacementsThisWeek !== 1 ? "s" : ""} cette semaine
              </span>
            ) : null}
          </div>

          {slots.length === 0 ? (
            <p className="text-sm text-slate-500 italic">
              Aucun créneau EDT pour cette classe. Les profs peuvent renseigner leur emploi du
              temps dans{" "}
              <Link href="/mon-planning" className="text-indigo-600 font-bold hover:underline">
                Mon planning
              </Link>
              .
            </p>
          ) : (
            <WeekGrid
              slots={gridSlots}
              renderCard={(slot) => {
                const full = slotById.get(slot.id)!;
                return (
                  <div
                    className={`rounded-lg border px-2 py-1.5 text-[11px] leading-snug ${
                      full.kind === "remplacement"
                        ? "bg-violet-50 border-violet-200"
                        : "bg-white border-indigo-100"
                    }`}
                  >
                    <p className="font-bold text-indigo-900">
                      {full.start}–{full.end}
                    </p>
                    <p className="text-slate-800 font-semibold">{full.subject}</p>
                    <p className="text-slate-500">
                      {full.teacherName}
                      {full.room ? ` · ${full.room}` : ""}
                    </p>
                    {full.kind === "remplacement" && full.replacementDate ? (
                      <p className="text-[10px] text-violet-700 font-bold mt-0.5">
                        Rempl. {full.replacementDate}
                      </p>
                    ) : null}
                  </div>
                );
              }}
            />
          )}

          <details className="text-xs text-slate-500">
            <summary className="cursor-pointer font-semibold text-slate-600">
              Détail par jour
            </summary>
            <ul className="mt-2 space-y-2">
              {([1, 2, 3, 4, 5] as PlanningWeekday[]).map((day) => {
                const daySlots = slots.filter((s) => s.day === day);
                return (
                  <li key={day}>
                    <span className="font-bold text-slate-700">
                      {PLANNING_WEEKDAY_LABELS[day]}
                    </span>
                    {daySlots.length === 0 ? (
                      <span className="ml-2 italic">—</span>
                    ) : (
                      <ul className="ml-3 mt-0.5 space-y-0.5">
                        {daySlots.map((s) => (
                          <li key={s.id}>
                            {s.start}–{s.end} {s.subject} ({s.teacherName})
                          </li>
                        ))}
                      </ul>
                    )}
                  </li>
                );
              })}
            </ul>
          </details>
        </>
      ) : !loading && !classe && classes.length === 0 ? (
        <p className="text-sm text-slate-500 italic">
          Aucune classe disponible — renseignez les EDT profs ou le roster établissement.
        </p>
      ) : !loading && !classe ? (
        <p className="text-sm text-slate-500">Sélectionnez une classe pour afficher l’EDT.</p>
      ) : null}
    </div>
  );
}
