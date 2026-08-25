"use client";

import { useEffect, useState } from "react";
import {
  ClassAllocationAlert,
  ClassAllocationCard,
  ClassAllocationShell,
} from "@/app/components/class-allocation/ClassAllocationShell";
import { classLevelLabel } from "@/app/lib/class-allocation-labels";
import { CLASS_LEVELS } from "@/app/lib/class-allocation-types";

type Run = {
  id: string;
  createdAt: string;
  score: number;
  diagnostics: string[];
  levelResults: Record<string, { className: string; studentInes: string[]; stats: { count: number; avgScore: number; girls: number; boys: number; other?: number } }[]>;
};

function genderBalance(girls: number, boys: number): string {
  const total = girls + boys;
  if (!total) return "—";
  const girlsPct = Math.round((girls / total) * 100);
  return `${girlsPct}% F / ${100 - girlsPct}% G`;
}

export default function ClassAllocationResultsPage() {
  const [run, setRun] = useState<Run | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void (async () => {
      try {
        const res = await fetch("/api/toolbox/class-allocation/latest", { cache: "no-store" });
        const j = await res.json();
        if (res.ok && j.run) setRun(j.run as Run);
      } catch {
        /* ignore */
      }
    })();
  }, []);

  async function generate() {
    setBusy(true);
    setMsg(null);
    try {
      const res = await fetch("/api/toolbox/class-allocation/run", { method: "POST" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setRun(j.run || null);
      setMsg("Proposition générée.");
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function rerun(level: string) {
    setBusy(true);
    setMsg(null);
    try {
      const res = await fetch("/api/toolbox/class-allocation/rerun-level", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ level }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setRun(j.run || null);
      setMsg(`Niveau ${classLevelLabel(level)} recalculé.`);
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function publish() {
    if (!run) return;
    const ok = window.confirm(
      "Publier cette proposition dans le registre élèves ?\nLes classes des élèves concernés seront mises à jour (dossiers, notes, appels, facturation).",
    );
    if (!ok) return;
    setBusy(true);
    setMsg(null);
    try {
      const res = await fetch("/api/toolbox/class-allocation/publish", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ runId: run.id }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur publication");
      const missing =
        Array.isArray(j.missingInes) && j.missingInes.length > 0
          ? ` ${j.missingInes.length} INE introuvable(s) dans le registre.`
          : "";
      setMsg((j.message as string) + missing);
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <ClassAllocationShell
      badge="Direction"
      title="Résultats de répartition"
      subtitle="Comparez les propositions, vérifiez l'équilibre filles/garçons, puis publiez dans le registre élèves."
      backHref="/toolbox/repartition-classes"
    >
      <div className="mb-6 flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={() => void generate()}
          className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
        >
          {busy ? "Calcul…" : "Générer une proposition"}
        </button>
        <button
          type="button"
          disabled={busy || !run}
          onClick={() => void publish()}
          className="rounded-xl bg-emerald-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
        >
          Publier dans le registre
        </button>
        {CLASS_LEVELS.map((l) => (
          <button
            key={l}
            type="button"
            disabled={busy}
            onClick={() => void rerun(l)}
            className="rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm font-bold text-slate-700 disabled:opacity-50"
          >
            Recalculer {classLevelLabel(l)}
          </button>
        ))}
      </div>

      {msg && <ClassAllocationAlert tone={msg.includes("Erreur") ? "error" : "success"}>{msg}</ClassAllocationAlert>}

      {run && (
        <div className="space-y-6">
          <ClassAllocationCard>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="text-xs font-bold uppercase tracking-wide text-slate-500">Run {run.id.slice(0, 8)}</p>
                <p className="text-sm text-slate-600">{new Date(run.createdAt).toLocaleString("fr-FR")}</p>
              </div>
              <p className="rounded-full bg-indigo-50 px-4 py-2 text-sm font-bold text-indigo-800">
                Score global : {run.score}
              </p>
            </div>
          </ClassAllocationCard>

          {Object.entries(run.levelResults)
            .filter(([, classes]) => classes.length > 0)
            .map(([level, classes]) => (
              <ClassAllocationCard key={level} title={classLevelLabel(level)}>
                <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                  {classes.map((c) => (
                    <div key={c.className} className="rounded-xl border border-slate-100 bg-gradient-to-br from-white to-slate-50 p-4">
                      <p className="text-lg font-black text-slate-900">{c.className}</p>
                      <dl className="mt-3 space-y-1 text-sm text-slate-600">
                        <div className="flex justify-between"><dt>Effectif</dt><dd className="font-bold text-slate-900">{c.stats.count}</dd></div>
                        <div className="flex justify-between"><dt>Équilibre F/G</dt><dd className="font-bold text-slate-900">{genderBalance(c.stats.girls, c.stats.boys)}</dd></div>
                        <div className="flex justify-between"><dt>Moyenne</dt><dd className="font-bold text-slate-900">{c.stats.avgScore}</dd></div>
                      </dl>
                    </div>
                  ))}
                </div>
              </ClassAllocationCard>
            ))}

          {run.diagnostics.length > 0 && (
            <ClassAllocationCard title="Contraintes non satisfaites (extrait)">
              <ul className="max-h-48 space-y-1 overflow-y-auto text-sm text-slate-600">
                {run.diagnostics.slice(0, 30).map((d) => (
                  <li key={d} className="rounded-lg bg-amber-50 px-3 py-1.5 text-amber-900">{d}</li>
                ))}
              </ul>
            </ClassAllocationCard>
          )}
        </div>
      )}
    </ClassAllocationShell>
  );
}
