"use client";

import { useCallback, useEffect, useState } from "react";
import type { InternatSortieRow } from "@/app/lib/internat-sorties-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

function todayParis(): string {
  return new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

function addDays(iso: string, n: number): string {
  const [y, m, d] = iso.split("-").map(Number) as [number, number, number];
  const dt = new Date(Date.UTC(y, m - 1, d));
  dt.setUTCDate(dt.getUTCDate() + n);
  return dt.toISOString().slice(0, 10);
}

export default function InternatSortiesClient() {
  const [sorties, setSorties] = useState<InternatSortieRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [debut, setDebut] = useState(todayParis);
  const [fin, setFin] = useState(() => addDays(todayParis(), 2));
  const [motif, setMotif] = useState("Week-end famille");
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setError(null);
    const res = await fetch("/api/internat/sorties", { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      error?: string;
      sorties?: InternatSortieRow[];
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setSorties(data.sorties ?? []);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (q.trim().length < 2 || selected) {
      setHits([]);
      return;
    }
    const t = setTimeout(() => {
      void (async () => {
        const res = await fetch(`/api/internat/sorties?q=${encodeURIComponent(q.trim())}`, {
          credentials: "include",
        });
        const data = (await res.json().catch(() => ({}))) as { eleves?: EleveLite[] };
        if (res.ok) setHits(data.eleves ?? []);
      })();
    }, 220);
    return () => clearTimeout(t);
  }, [q, selected]);

  async function creer() {
    if (!selected) {
      setError("Choisissez un élève.");
      return;
    }
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/internat/sorties", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "create",
          eleveId: selected.id,
          dateDebut: debut,
          dateFin: fin,
          motif,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        sortie?: InternatSortieRow;
      };
      if (!res.ok) {
        setError(data.error || "Création impossible.");
        return;
      }
      const s = data.sortie;
      setOkMsg(
        s
          ? `${s.elevePrenom} ${s.eleveNom} — sortie ${s.dateDebut} → ${s.dateFin}.`
          : "Sortie enregistrée.",
      );
      setSelected(null);
      setQ("");
      await load();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <p className="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
        Sortie week-end / correspondant (Postgres). Exclut l’élève du roster d’appel du soir sur
        la période. JSON hub intact.
      </p>

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Nouvelle sortie
        </h2>
        <div className="mt-4 space-y-3">
          <label className="block text-sm font-semibold text-slate-800">
            Élève
            <input
              value={selected ? `${selected.prenom} ${selected.nom}` : q}
              onChange={(e) => {
                setSelected(null);
                setQ(e.target.value);
              }}
              placeholder="Nom ou prénom"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          {!selected && hits.length > 0 ? (
            <ul className="overflow-hidden rounded-xl border border-slate-200">
              {hits.map((e) => (
                <li key={e.id}>
                  <button
                    type="button"
                    className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-slate-50"
                    onClick={() => {
                      setSelected(e);
                      setQ("");
                      setHits([]);
                    }}
                  >
                    <span className="font-semibold">
                      {e.prenom} {e.nom}
                    </span>
                    <span className="text-xs text-slate-500">{e.classe || "—"}</span>
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="text-sm font-semibold text-slate-800">
              Début
              <input
                type="date"
                value={debut}
                onChange={(e) => setDebut(e.target.value)}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              />
            </label>
            <label className="text-sm font-semibold text-slate-800">
              Fin
              <input
                type="date"
                value={fin}
                onChange={(e) => setFin(e.target.value)}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              />
            </label>
          </div>
          <label className="block text-sm font-semibold text-slate-800">
            Motif
            <input
              value={motif}
              onChange={(e) => setMotif(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <button
            type="button"
            disabled={busy || !selected}
            onClick={() => void creer()}
            className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer la sortie
          </button>
        </div>
      </section>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-900">
          {error}
        </p>
      ) : null}
      {okMsg ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900">
          {okMsg}
        </p>
      ) : null}

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Sorties</h2>
        {sorties.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucune sortie enregistrée.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {sorties.map((s) => (
              <li key={s.id} className="py-3 text-sm">
                <p className="font-semibold text-slate-900">
                  {s.elevePrenom} {s.eleveNom}
                  {s.eleveClasse ? (
                    <span className="ml-2 text-xs font-medium text-slate-500">{s.eleveClasse}</span>
                  ) : null}
                </p>
                <p className="text-xs text-slate-500">
                  {s.dateDebut} → {s.dateFin}
                  {s.motif ? ` · ${s.motif}` : ""}
                </p>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
