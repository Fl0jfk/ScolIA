"use client";

import Link from "next/link";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  ClassAllocationAlert,
  ClassAllocationCard,
  ClassAllocationShell,
} from "@/app/components/class-allocation/ClassAllocationShell";
import { classLevelLabel } from "@/app/lib/class-allocation-labels";
import { CLASS_LEVELS, type ClassLevel } from "@/app/lib/class-allocation-types";

type LevelCfg = {
  level: ClassLevel;
  sourceClassPrefixes: string[];
  targetClasses: string[];
};

type Cfg = {
  id: string;
  label: string;
  isOpen: boolean;
  levels: LevelCfg[];
};

export default function ClassAllocationHubPage() {
  const [cfg, setCfg] = useState<Cfg | null>(null);
  const [elevesCount, setElevesCount] = useState<number | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const classesInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void (async () => {
      const [cfgRes, rosterRes] = await Promise.all([
        fetch("/api/toolbox/class-allocation/config", { cache: "no-store" }),
        fetch("/api/settings/roster", { cache: "no-store" }),
      ]);
      const cfgJson = await cfgRes.json();
      const rosterJson = await rosterRes.json();
      setCfg(cfgJson.config || null);
      setElevesCount(rosterJson.elevesCount ?? null);
    })();
  }, []);

  async function save() {
    if (!cfg) return;
    const res = await fetch("/api/toolbox/class-allocation/config", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(cfg),
    });
    const j = await res.json();
    setCfg(j.config || cfg);
    setMsg(res.ok ? "Campagne enregistrée." : j.error || "Erreur");
  }

  async function importClasses(file: File) {
    setBusy(true);
    setMsg(null);
    try {
      const form = new FormData();
      form.append("file", file);
      const res = await fetch("/api/toolbox/class-allocation/classes/import", {
        method: "POST",
        body: form,
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur import");
      setCfg(j.config || cfg);
      setMsg(j.message || "Classes importées.");
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : "Erreur import");
    } finally {
      setBusy(false);
    }
  }

  async function fillFromEleves(level?: ClassLevel) {
    setBusy(true);
    setMsg(null);
    try {
      const res = await fetch("/api/toolbox/class-allocation/classes/from-eleves", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(level ? { level } : {}),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setCfg(j.config || cfg);
      setMsg(j.message || "Classes chargées.");
    } catch (e: unknown) {
      setMsg(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  const levelRows = useMemo(() => {
    const map = new Map(cfg?.levels.map((l) => [l.level, l]));
    return CLASS_LEVELS.map(
      (level) => map.get(level) || { level, sourceClassPrefixes: [], targetClasses: [] },
    );
  }, [cfg?.levels]);

  return (
    <ClassAllocationShell
      badge="Répartition des classes"
      title="Répartition des classes"
      subtitle="Campagne annuelle et lancement du moteur de répartition."
      backHref="/services"
    >
      <ClassAllocationAlert tone="info">
        Liste élèves, professeurs par classe et catalogue profs :{" "}
        <Link href="/parametres?tab=referentiel" className="font-bold underline">
          Paramètres généraux → Référentiel scolaire
        </Link>
        {elevesCount != null && ` (${elevesCount} élèves chargés).`}
      </ClassAllocationAlert>

      {cfg && (
        <div className="space-y-6">
          <ClassAllocationCard title="Campagne">
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">ID campagne</span>
                <input
                  className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                  value={cfg.id}
                  onChange={(e) => setCfg({ ...cfg, id: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Libellé</span>
                <input
                  className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                  value={cfg.label}
                  onChange={(e) => setCfg({ ...cfg, label: e.target.value })}
                />
              </label>
            </div>
            <label className="flex items-center gap-3 rounded-xl border border-slate-100 bg-slate-50 px-4 py-3 text-sm font-semibold">
              <input
                type="checkbox"
                checked={cfg.isOpen}
                onChange={(e) => setCfg({ ...cfg, isOpen: e.target.checked })}
              />
              Campagne ouverte aux parents
            </label>
          </ClassAllocationCard>

          <ClassAllocationCard title="Niveaux et classes">
            <p className="text-sm text-slate-600">
              Importez un fichier Excel avec les colonnes <strong>Niveau</strong> (École, Collège ou Lycée),{" "}
              <strong>Type</strong> (<em>actuelle</em> ou <em>cible</em>) et <strong>Classe</strong>.
            </p>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                disabled={busy}
                onClick={() => classesInputRef.current?.click()}
                className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
              >
                Importer Excel classes
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => void fillFromEleves()}
                className="rounded-xl border border-indigo-200 bg-indigo-50 px-4 py-2 text-sm font-bold text-indigo-900 disabled:opacity-50"
              >
                Classes actuelles depuis les élèves
              </button>
            </div>
            <input
              ref={classesInputRef}
              type="file"
              accept=".xlsx,.xls"
              className="hidden"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) void importClasses(f);
                e.target.value = "";
              }}
            />

            {levelRows.map((row) => (
              <div key={row.level} className="rounded-xl border border-slate-100 bg-slate-50/70 p-4 space-y-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <p className="text-sm font-bold text-slate-900">{classLevelLabel(row.level)}</p>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void fillFromEleves(row.level)}
                    className="text-xs font-bold text-indigo-700 underline disabled:opacity-50"
                  >
                    Actuelles depuis élèves
                  </button>
                </div>
                <div>
                  <p className="text-xs font-bold uppercase text-slate-500">
                    Classes actuelles ({row.sourceClassPrefixes.length})
                  </p>
                  <p className="mt-1 text-sm text-slate-800">
                    {row.sourceClassPrefixes.length ? row.sourceClassPrefixes.join(", ") : "—"}
                  </p>
                </div>
                <div>
                  <p className="text-xs font-bold uppercase text-slate-500">
                    Classes cibles ({row.targetClasses.length})
                  </p>
                  <p className="mt-1 text-sm text-slate-800">
                    {row.targetClasses.length ? row.targetClasses.join(", ") : "—"}
                  </p>
                </div>
              </div>
            ))}
          </ClassAllocationCard>

          <button
            type="button"
            onClick={() => void save()}
            className="rounded-xl bg-slate-900 px-5 py-2.5 text-sm font-bold text-white"
          >
            Enregistrer la campagne
          </button>
          {msg && (
            <ClassAllocationAlert tone={msg.toLowerCase().includes("erreur") ? "error" : "success"}>
              {msg}
            </ClassAllocationAlert>
          )}
        </div>
      )}

      <div className="mt-8 grid gap-3 sm:grid-cols-3">
        <Link
          href="/toolbox/repartition-classes/interne"
          className="rounded-2xl border border-slate-200 bg-white p-5 font-semibold shadow-sm hover:border-indigo-200 hover:bg-indigo-50/40"
        >
          Préparer la classe (prof)
        </Link>
        <Link
          href="/toolbox/repartition-classes/resultats"
          className="rounded-2xl border border-slate-200 bg-white p-5 font-semibold shadow-sm hover:border-indigo-200 hover:bg-indigo-50/40"
        >
          Résultats & publication
        </Link>
        <Link
          href="/repartition-classes"
          target="_blank"
          className="rounded-2xl border border-slate-200 bg-white p-5 font-semibold shadow-sm hover:border-indigo-200 hover:bg-indigo-50/40"
        >
          Page parents
        </Link>
      </div>
    </ClassAllocationShell>
  );
}
