"use client";

import { useCallback, useEffect, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import type { AccueilBoardRow, AccueilPeriodMode, AccueilSearchHit } from "@/app/lib/accueil-absences-types";

function todayIso(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function kindBadge(kind: AccueilBoardRow["kind"] | AccueilSearchHit["kind"]): string {
  if (kind === "eleve") return "Élève";
  if (kind === "enseignant" || kind === "professeur") return "Professeur";
  return "Personnel OGEC";
}

export default function AccueilAbsencesClient() {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<AccueilSearchHit[]>([]);
  const [selected, setSelected] = useState<AccueilSearchHit | null>(null);
  const [mode, setMode] = useState<AccueilPeriodMode>("today");
  const [startDate, setStartDate] = useState(todayIso());
  const [endDate, setEndDate] = useState(todayIso());
  const [startTime, setStartTime] = useState("08:00");
  const [endTime, setEndTime] = useState("12:00");
  const [motif, setMotif] = useState("");
  const [rows, setRows] = useState<AccueilBoardRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const loadBoard = useCallback(async () => {
    const res = await fetch("/api/accueil/absences", { cache: "no-store" });
    const data = (await res.json()) as { error?: string; rows?: AccueilBoardRow[] };
    if (!res.ok) throw new Error(data.error || "Chargement impossible");
    setRows(data.rows || []);
  }, []);

  useEffect(() => {
    void loadBoard().catch((e: unknown) => {
      setError(e instanceof Error ? e.message : "Erreur");
    });
  }, [loadBoard]);

  useEffect(() => {
    if (selected) return;
    if (q.trim().length < 3) {
      setHits([]);
      return;
    }
    const t = setTimeout(() => {
      void (async () => {
        const res = await fetch(`/api/accueil/absences/search?q=${encodeURIComponent(q)}`, {
          cache: "no-store",
        });
        const data = (await res.json()) as { hits?: AccueilSearchHit[] };
        if (res.ok) setHits(data.hits || []);
      })();
    }, 180);
    return () => clearTimeout(t);
  }, [q, selected]);

  const resetForm = () => {
    setSelected(null);
    setQ("");
    setHits([]);
    setMode("today");
    setStartDate(todayIso());
    setEndDate(todayIso());
    setStartTime("08:00");
    setEndTime("12:00");
    setMotif("");
  };

  const submit = async () => {
    if (!selected) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/accueil/absences", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          kind: selected.kind,
          subjectId: selected.id,
          mode,
          startDate,
          endDate: mode === "multi_day" ? endDate : startDate,
          startTime: mode === "hours" ? startTime : null,
          endTime: mode === "hours" ? endTime : null,
          motif: motif.trim() || null,
          canal: "telephone",
        }),
      });
      const data = (await res.json()) as {
        error?: string;
        displayName?: string;
        pendingDirection?: boolean;
      };
      if (!res.ok) throw new Error(data.error || "Enregistrement impossible");
      const name = data.displayName || selected.displayName;
      const isProf = selected.kind === "enseignant" || selected.scope === "professeur";
      if (data.pendingDirection) {
        setMessage(
          isProf
            ? `${name} — transmis à la direction. Après validation : calendrier absences professeurs + mail à la personne qui déclare au rectorat.`
            : `${name} — transmis à la direction. Après validation : calendrier + comptabilité RH.`,
        );
      } else {
        setMessage(`${name} — absence enregistrée.`);
      }
      resetForm();
      await loadBoard();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const annuler = async (id: string) => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/accueil/absences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "annuler", id }),
      });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Annulation impossible");
      setMessage("Absence annulée.");
      await loadBoard();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <ModulePageShell>
      <ModulePageHeader
        eyebrow="Standard"
        title="Absence accueil"
        description="Téléphone à l’oreille : 3 lettres, on déclare. Élèves tout de suite. Professeurs : validation direction, puis calendrier absences profs et mail à la personne qui déclare au rectorat. Personnel OGEC : circuit RH / compta."
      />

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
        <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm space-y-4">
          <label className="block">
            <span className="text-xs font-semibold uppercase tracking-wide text-slate-500">
              Qui est absent ?
            </span>
            {selected ? (
              <div className="mt-2 flex items-center justify-between gap-3 rounded-2xl border border-indigo-200 bg-indigo-50/60 px-4 py-3">
                <div>
                  <p className="font-semibold text-slate-900">{selected.displayName}</p>
                  <p className="text-sm text-slate-600">
                    {kindBadge(selected.kind)}
                    {selected.subtitle ? ` · ${selected.subtitle}` : ""}
                  </p>
                </div>
                <button
                  type="button"
                  className="text-sm font-semibold text-indigo-700 hover:underline"
                  onClick={resetForm}
                >
                  Changer
                </button>
              </div>
            ) : (
              <input
                value={q}
                onChange={(e) => setQ(e.target.value)}
                placeholder="Nom ou prénom (3 lettres)…"
                autoComplete="off"
                className="mt-2 w-full rounded-2xl border border-slate-200 px-4 py-3 text-base outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100"
              />
            )}
          </label>

          {!selected && hits.length > 0 ? (
            <ul className="divide-y divide-slate-100 rounded-2xl border border-slate-200 overflow-hidden">
              {hits.map((h) => (
                <li key={`${h.kind}-${h.id}`}>
                  <button
                    type="button"
                    className="flex w-full items-start gap-3 px-4 py-3 text-left hover:bg-slate-50"
                    onClick={() => {
                      setSelected(h);
                      setHits([]);
                      setQ(h.displayName);
                    }}
                  >
                    <span className="mt-0.5 rounded-full bg-slate-100 px-2 py-0.5 text-[11px] font-bold uppercase text-slate-600">
                      {kindBadge(h.kind)}
                    </span>
                    <span>
                      <span className="block font-semibold text-slate-900">{h.displayName}</span>
                      <span className="block text-sm text-slate-500">{h.subtitle}</span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          ) : null}

          {selected ? (
            <>
              <div className="flex flex-wrap gap-2">
                {(
                  [
                    ["today", "Aujourd’hui"],
                    ["hours", "Horaires"],
                    ["multi_day", "Plusieurs jours"],
                  ] as const
                ).map(([value, label]) => (
                  <button
                    key={value}
                    type="button"
                    onClick={() => setMode(value)}
                    className={`rounded-full px-3 py-1.5 text-sm font-semibold ${
                      mode === value
                        ? "bg-indigo-600 text-white"
                        : "border border-slate-200 bg-white text-slate-700 hover:bg-slate-50"
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>

              {mode === "hours" ? (
                <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                  <label className="text-sm">
                    Date
                    <input
                      type="date"
                      value={startDate}
                      onChange={(e) => setStartDate(e.target.value)}
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                    />
                  </label>
                  <label className="text-sm">
                    Début
                    <input
                      type="time"
                      value={startTime}
                      onChange={(e) => setStartTime(e.target.value)}
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                    />
                  </label>
                  <label className="text-sm">
                    Fin
                    <input
                      type="time"
                      value={endTime}
                      onChange={(e) => setEndTime(e.target.value)}
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                    />
                  </label>
                </div>
              ) : null}

              {mode === "multi_day" ? (
                <div className="grid grid-cols-2 gap-3">
                  <label className="text-sm">
                    Du
                    <input
                      type="date"
                      value={startDate}
                      onChange={(e) => setStartDate(e.target.value)}
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                    />
                  </label>
                  <label className="text-sm">
                    Au
                    <input
                      type="date"
                      value={endDate}
                      onChange={(e) => setEndDate(e.target.value)}
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                    />
                  </label>
                </div>
              ) : null}

              <label className="block text-sm">
                Motif (optionnel)
                <input
                  value={motif}
                  onChange={(e) => setMotif(e.target.value)}
                  placeholder="RDV médical, parents ont appelé…"
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                />
              </label>

              <button
                type="button"
                disabled={busy}
                onClick={() => void submit()}
                className="w-full rounded-2xl bg-indigo-600 px-4 py-3 text-sm font-bold text-white hover:bg-indigo-700 disabled:opacity-50"
              >
                {busy ? "Enregistrement…" : "Déclarer l’absence"}
              </button>
              {selected.kind === "enseignant" || selected.scope === "professeur" ? (
                <p className="text-xs text-slate-500">
                  La direction valide d’abord. Ensuite l’absence apparaît au calendrier professeurs et
                  un e-mail part à la personne qui déclare au rectorat (réglages Notifications).
                </p>
              ) : selected.kind === "personnel" ? (
                <p className="text-xs text-slate-500">
                  Circuit RH : validation direction, puis calendrier et comptabilité.
                </p>
              ) : (
                <p className="text-xs text-slate-500">
                  Enregistré tout de suite pour la vie scolaire (pas de validation direction).
                </p>
              )}
            </>
          ) : (
            <p className="text-sm text-slate-500">Tapez au moins 3 lettres d’un nom ou prénom.</p>
          )}

          {error ? <p className="text-sm font-medium text-rose-600">{error}</p> : null}
          {message ? <p className="text-sm font-medium text-emerald-700">{message}</p> : null}
        </section>

        <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-base font-semibold text-slate-900">Aujourd’hui</h2>
          <p className="mt-1 text-sm text-slate-500">
            Visible selon les droits : élèves et profs pour les CPE ; personnel OGEC pour l’admin, la
            compta et la direction.
          </p>
          <ul className="mt-4 space-y-2">
            {rows.length === 0 ? (
              <li className="rounded-2xl border border-dashed border-slate-200 px-4 py-6 text-sm text-slate-500">
                Aucune absence déclarée à l’accueil pour aujourd’hui.
              </li>
            ) : (
              rows.map((r) => (
                <li
                  key={`${r.kind}-${r.id}`}
                  className="flex items-start justify-between gap-3 rounded-2xl border border-slate-100 bg-slate-50/80 px-4 py-3"
                >
                  <div>
                    <p className="font-semibold text-slate-900">{r.displayName}</p>
                    <p className="text-sm text-slate-600">
                      {kindBadge(r.kind)} · {r.subtitle}
                    </p>
                    {r.motif ? <p className="mt-0.5 text-xs text-slate-500">{r.motif}</p> : null}
                  </div>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void annuler(r.id)}
                    className="shrink-0 text-xs font-semibold text-rose-600 hover:underline disabled:opacity-50"
                  >
                    Annuler
                  </button>
                </li>
              ))
            )}
          </ul>
        </section>
      </div>
    </ModulePageShell>
  );
}
