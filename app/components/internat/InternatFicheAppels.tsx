"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { parisDateKey } from "@/app/lib/paris-time";
import {
  countAppelMarks,
  filterAppels,
  monthBounds,
  monthCells,
  schoolYearMonthsUpTo,
  schoolYearOf,
  shiftMonth,
  type InternatAppelEntry,
} from "@/app/lib/internat-suivi-shared";
import { INTERNAT_ROLL_MARK_LABELS, type InternatRollMark } from "@/app/lib/internat-types";

const MARKS: Record<InternatRollMark, { symbol: string; tone: string }> = {
  present: { symbol: "✓", tone: "bg-emerald-100 text-emerald-800" },
  absent: { symbol: "✕", tone: "bg-red-100 text-red-800" },
  excuse: { symbol: "E", tone: "bg-amber-100 text-amber-800" },
  activite: { symbol: "A", tone: "bg-sky-100 text-sky-800" },
};

const PAGE_SIZE = 15;

async function fetchMonth(studentId: string, month: string): Promise<InternatAppelEntry[]> {
  const { from, to } = monthBounds(month);
  const params = new URLSearchParams({ studentId, from, to, limit: "120" });
  const res = await fetch(`/api/internat/roll-call/history?${params}`, { cache: "no-store" });
  const data = await res.json();
  if (!res.ok) throw new Error(data?.error || "Chargement des appels impossible.");
  return ((data.history || []) as InternatAppelEntry[]).map((r) => ({
    ...r,
    markLabel: r.markLabel || INTERNAT_ROLL_MARK_LABELS[r.mark] || r.mark,
  }));
}

function longDay(date: string) {
  return new Date(`${date}T12:00:00`).toLocaleDateString("fr-FR", {
    weekday: "short",
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

/** Appels validés d'un interne : calendrier du mois, ou liste cherchable et triable. */
export default function InternatFicheAppels({ studentId }: { studentId: string }) {
  const today = parisDateKey(new Date());
  const currentMonth = today.slice(0, 7);
  const [view, setView] = useState<"calendrier" | "liste">("calendrier");
  const [month, setMonth] = useState(currentMonth);
  const [range, setRange] = useState<"mois" | "annee">("mois");
  const [byMonth, setByMonth] = useState<Record<string, InternatAppelEntry[]>>({});
  const [failure, setFailure] = useState<{ month: string; message: string } | null>(null);
  const [day, setDay] = useState<string | null>(null);
  const [mark, setMark] = useState<InternatRollMark | "all">("all");
  const [query, setQuery] = useState("");
  const [ascending, setAscending] = useState(false);
  const [shown, setShown] = useState(PAGE_SIZE);
  const requested = useRef(new Set<string>());

  // Année scolaire du mois affiché, jusqu'à aujourd'hui (ou jusqu'à juillet si elle est passée).
  const yearEnd = `${schoolYearOf(`${month}-01`) + 1}-07`;
  const yearMonths = schoolYearMonthsUpTo(yearEnd < currentMonth ? yearEnd : currentMonth);
  const wanted = range === "annee" ? yearMonths : [month];
  const wantedKey = wanted.join(",");

  const load = useCallback(
    async (months: string[]) => {
      for (const m of months) {
        if (requested.current.has(m)) continue;
        requested.current.add(m);
        try {
          const rows = await fetchMonth(studentId, m);
          setByMonth((prev) => ({ ...prev, [m]: rows }));
        } catch (e) {
          requested.current.delete(m);
          setFailure({
            month: m,
            message: e instanceof Error ? e.message : "Chargement des appels impossible.",
          });
          return;
        }
      }
    },
    [studentId],
  );

  useEffect(() => {
    setFailure(null);
    void load(wantedKey.split(","));
  }, [load, wantedKey]);

  useEffect(() => setShown(PAGE_SIZE), [mark, query, ascending, range, month]);

  const error = failure && wanted.includes(failure.month) ? failure.message : "";
  const loaded = wanted.filter((m) => byMonth[m]);
  const loading = !error && loaded.length < wanted.length;
  const entries = loaded.flatMap((m) => byMonth[m] || []);
  const counts = countAppelMarks(entries);
  const list = filterAppels(entries, { mark, query, ascending });
  const dayEntries = day ? entries.filter((e) => e.date === day) : [];
  const monthLabel = new Date(`${month}-01T12:00:00`).toLocaleDateString("fr-FR", {
    month: "long",
    year: "numeric",
  });

  const goMonth = (delta: number) => {
    setMonth(shiftMonth(month, delta));
    setDay(null);
  };

  return (
    <section className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Segmented
          value={view}
          options={[
            ["calendrier", "Calendrier"],
            ["liste", "Liste"],
          ]}
          onChange={(v) => {
            setView(v);
            if (v === "calendrier") setRange("mois");
          }}
        />
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => goMonth(-1)}
            className="h-9 w-9 rounded-lg border border-slate-200 font-bold"
            aria-label="Mois précédent"
          >
            ‹
          </button>
          <span className="min-w-[9.5rem] text-center text-sm font-black capitalize text-slate-900">
            {monthLabel}
          </span>
          <button
            type="button"
            onClick={() => goMonth(1)}
            disabled={month >= currentMonth}
            className="h-9 w-9 rounded-lg border border-slate-200 font-bold disabled:opacity-30"
            aria-label="Mois suivant"
          >
            ›
          </button>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-2">
        {(Object.keys(MARKS) as InternatRollMark[]).map((k) => (
          <button
            key={k}
            type="button"
            onClick={() => {
              setMark(mark === k ? "all" : k);
              setView("liste");
            }}
            className={`rounded-xl px-2 py-2 text-center ${MARKS[k].tone} ${
              view === "liste" && mark === k ? "ring-2 ring-slate-900" : ""
            }`}
          >
            <span className="block text-lg font-black">{counts[k]}</span>
            <span className="block text-[11px] font-bold">
              {k === "activite" ? "Activité" : INTERNAT_ROLL_MARK_LABELS[k]}
            </span>
          </button>
        ))}
      </div>
      <p className="text-xs text-slate-500">
        {range === "annee"
          ? `Totaux de l'année scolaire ${schoolYearOf(`${month}-01`)}-${schoolYearOf(`${month}-01`) + 1}`
          : "Totaux du mois affiché"}
        {" — appels validés uniquement."}
        {loading && ` Chargement… (${loaded.length}/${wanted.length})`}
      </p>

      {error && (
        <p className="text-sm text-red-700">
          {error}{" "}
          <button
            type="button"
            className="font-bold underline"
            onClick={() => {
              setFailure(null);
              void load(wanted);
            }}
          >
            Réessayer
          </button>
        </p>
      )}

      {view === "calendrier" ? (
        <>
          <div className="grid grid-cols-7 gap-1 text-center">
            {["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"].map((d) => (
              <span key={d} className="text-[11px] font-bold text-slate-400">
                {d}
              </span>
            ))}
            {monthCells(month).map((date, i) => {
              if (!date) return <span key={`vide-${i}`} />;
              const items = entries
                .filter((e) => e.date === date)
                .sort((a, b) => a.period.localeCompare(b.period));
              return (
                <button
                  key={date}
                  type="button"
                  disabled={!items.length}
                  onClick={() => setDay(date)}
                  aria-label={`${longDay(date)} : ${
                    items.map((e) => `${e.period} ${e.markLabel}`).join(", ") || "pas d'appel"
                  }`}
                  className={`min-h-[52px] rounded-lg border p-1 flex flex-col items-center gap-1 ${
                    day === date ? "border-slate-900 ring-1 ring-slate-900" : "border-slate-200"
                  } ${items.length ? "bg-white hover:bg-slate-50" : "bg-slate-50/60"} `}
                >
                  <span
                    className={`text-xs ${
                      date === today ? "font-black text-indigo-700" : "text-slate-500"
                    }`}
                  >
                    {Number(date.slice(8))}
                  </span>
                  <span className="flex gap-0.5">
                    {items.map((e) => (
                      <span
                        key={e.period}
                        className={`h-5 w-5 rounded-full text-xs font-black leading-5 ${MARKS[e.mark]?.tone}`}
                      >
                        {MARKS[e.mark]?.symbol}
                      </span>
                    ))}
                  </span>
                </button>
              );
            })}
          </div>
          {day ? (
            <div className="space-y-2">
              {dayEntries.map((e) => (
                <AppelRow key={e.period} entry={e} />
              ))}
            </div>
          ) : (
            <p className="text-xs text-slate-400">
              {!loading && !error && !entries.length
                ? "Aucun appel validé ce mois-ci."
                : "Cliquez sur un jour pour voir le détail (matin en premier quand il y a deux appels)."}
            </p>
          )}
        </>
      ) : (
        <>
          <div className="flex flex-wrap gap-2">
            <Segmented
              value={range}
              options={[
                ["mois", "Ce mois"],
                ["annee", "Toute l'année"],
              ]}
              onChange={setRange}
            />
            <select
              className="border rounded-xl px-3 py-2 text-sm"
              value={mark}
              onChange={(e) => setMark(e.target.value as InternatRollMark | "all")}
              aria-label="Filtrer par statut"
            >
              <option value="all">Tous les statuts</option>
              {(Object.keys(MARKS) as InternatRollMark[]).map((k) => (
                <option key={k} value={k}>
                  {INTERNAT_ROLL_MARK_LABELS[k]}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={() => setAscending(!ascending)}
              className="border rounded-xl px-3 py-2 text-sm font-semibold"
            >
              {ascending ? "Plus anciens d'abord ↑" : "Plus récents d'abord ↓"}
            </button>
            <input
              type="search"
              className="border rounded-xl px-3 py-2 text-sm flex-1 min-w-[10rem]"
              placeholder="Chercher (date, jour, motif…)"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            {list.slice(0, shown).map((e) => (
              <AppelRow key={`${e.date}-${e.period}`} entry={e} />
            ))}
          </div>
          {!loading && !error && !list.length && (
            <p className="text-sm text-slate-500">Aucun appel ne correspond.</p>
          )}
          {list.length > shown && (
            <button
              type="button"
              onClick={() => setShown(shown + PAGE_SIZE)}
              className="w-full border rounded-xl py-2 text-sm font-bold text-slate-600"
            >
              Voir plus ({list.length - shown} restants)
            </button>
          )}
        </>
      )}
    </section>
  );
}

function AppelRow({ entry }: { entry: InternatAppelEntry }) {
  return (
    <div className="flex items-center justify-between gap-2 rounded-xl border border-slate-200 px-3 py-2 text-sm">
      <div className="min-w-0">
        <p className="font-semibold text-slate-900">{longDay(entry.date)}</p>
        <p className="text-xs text-slate-500">
          Appel du {entry.period}
          {entry.validatedBy ? ` · validé par ${entry.validatedBy}` : ""}
        </p>
        {entry.note && <p className="text-xs text-slate-600">{entry.note}</p>}
      </div>
      <span className={`shrink-0 text-xs font-bold px-2.5 py-1 rounded-lg ${MARKS[entry.mark]?.tone}`}>
        {MARKS[entry.mark]?.symbol} {entry.markLabel}
      </span>
    </div>
  );
}

function Segmented<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: Array<[T, string]>;
  onChange: (v: T) => void;
}) {
  return (
    <div className="inline-flex rounded-xl border border-slate-200 p-0.5">
      {options.map(([id, label]) => (
        <button
          key={id}
          type="button"
          onClick={() => onChange(id)}
          className={`px-3 py-1.5 rounded-lg text-sm font-bold ${
            value === id ? "bg-slate-900 text-white" : "text-slate-500"
          }`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
