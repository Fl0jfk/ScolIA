"use client";

import { useCallback, useEffect, useState } from "react";

type Annee = {
  id: string;
  label: string;
  startsOn: string | null;
  endsOn: string | null;
  isCurrent: boolean;
};

function nextYearLabel(from: string | null): string {
  if (from && /^\d{4}-\d{4}$/.test(from)) {
    const y = Number(from.slice(0, 4));
    return `${y + 1}-${y + 2}`;
  }
  const y = new Date().getFullYear();
  const m = new Date().getMonth();
  const start = m >= 7 ? y : y - 1;
  return `${start + 1}-${start + 2}`;
}

export default function AnneeScolairePanel() {
  const [annees, setAnnees] = useState<Annee[]>([]);
  const [current, setCurrent] = useState<Annee | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [label, setLabel] = useState("");

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/annees-scolaires", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setAnnees(data.annees || []);
      setCurrent(data.current || null);
      setLabel((prev) => prev || nextYearLabel(data.current?.label ?? null));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const post = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/annees-scolaires", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      setMessage(
        body.action === "setCurrent"
          ? `Année ${data.annee?.label} définie comme courante.`
          : `Année ${data.annee?.label} enregistrée.`,
      );
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="font-black text-slate-900">Année scolaire courante</h2>
        <p className="text-sm text-slate-600">
          Référence globale pour notes, groupes, scolarité, facturation et exports Siècle. Une seule année
          est « courante » à la fois.
        </p>
        {current ? (
          <p className="rounded-xl bg-indigo-50 border border-indigo-100 px-4 py-3 text-sm font-bold text-indigo-900">
            {current.label}
            {current.startsOn && current.endsOn
              ? ` · du ${current.startsOn} au ${current.endsOn}`
              : ""}
          </p>
        ) : (
          <p className="text-sm text-amber-800 bg-amber-50 border border-amber-100 rounded-xl px-4 py-3">
            Aucune année courante — ouvrez une année ci-dessous.
          </p>
        )}
      </section>

      {message && <p className="text-sm font-semibold text-emerald-700">{message}</p>}
      {error && <p className="text-sm text-red-600">{error}</p>}

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
        <h2 className="font-black text-slate-900">Ouvrir / mettre à jour une année</h2>
        <div className="flex flex-wrap gap-2 items-end">
          <label className="text-sm">
            <span className="block text-xs font-bold text-slate-500 mb-1">Libellé AAAA-AAAA</span>
            <input
              className="border rounded-xl px-3 py-2 font-mono"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="2026-2027"
            />
          </label>
          <button
            type="button"
            disabled={busy || !label.trim()}
            onClick={() =>
              void post({ action: "openYear", label: label.trim(), makeCurrent: true })
            }
            className="rounded-xl bg-indigo-600 text-white px-4 py-2 text-sm font-bold disabled:opacity-50"
          >
            Ouvrir et activer
          </button>
          <button
            type="button"
            disabled={busy || !label.trim()}
            onClick={() =>
              void post({ action: "upsert", label: label.trim(), makeCurrent: false })
            }
            className="rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-bold text-slate-700 disabled:opacity-50"
          >
            Créer sans activer
          </button>
        </div>
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="font-black text-slate-900">Historique ({annees.length})</h2>
        <ul className="divide-y text-sm">
          {annees.map((a) => (
            <li key={a.id} className="py-3 flex flex-wrap items-center justify-between gap-2">
              <div>
                <span className="font-bold text-slate-900">{a.label}</span>
                {a.isCurrent ? (
                  <span className="ml-2 rounded bg-indigo-100 px-1.5 py-0.5 text-[10px] font-bold uppercase text-indigo-800">
                    Courante
                  </span>
                ) : null}
                <p className="text-xs text-slate-500">
                  {a.startsOn || "—"} → {a.endsOn || "—"}
                </p>
              </div>
              {!a.isCurrent ? (
                <button
                  type="button"
                  disabled={busy}
                  className="text-xs font-bold text-indigo-600 disabled:opacity-50"
                  onClick={() => void post({ action: "setCurrent", anneeId: a.id })}
                >
                  Définir comme courante
                </button>
              ) : null}
            </li>
          ))}
          {!annees.length && (
            <li className="py-3 text-slate-500">Aucune année enregistrée.</li>
          )}
        </ul>
      </section>
    </div>
  );
}
