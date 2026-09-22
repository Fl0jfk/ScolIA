"use client";

import { useCallback, useEffect, useState } from "react";
import type { InfirmeriePassageRow } from "@/app/lib/infirmerie-passages-db";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

function formatHeure(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString("fr-FR", {
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

export default function InfirmeriePassagesClient() {
  const [passages, setPassages] = useState<InfirmeriePassageRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [motif, setMotif] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const res = await fetch("/api/sante/passages", { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      passages?: InfirmeriePassageRow[];
      error?: string;
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setPassages(data.passages ?? []);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (q.trim().length < 2) {
      setHits([]);
      return;
    }
    const t = setTimeout(() => {
      void (async () => {
        const res = await fetch(`/api/sante/passages?q=${encodeURIComponent(q.trim())}`, {
          credentials: "include",
        });
        const data = (await res.json().catch(() => ({}))) as { eleves?: EleveLite[] };
        if (res.ok) setHits(data.eleves ?? []);
      })();
    }, 220);
    return () => clearTimeout(t);
  }, [q]);

  async function arriver() {
    if (!selected) {
      setError("Choisissez un élève.");
      return;
    }
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/sante/passages", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ eleveId: selected.id, motifCourt: motif }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Enregistrement impossible.");
        return;
      }
      setOkMsg(`${selected.prenom} ${selected.nom} est à l’infirmerie.`);
      setSelected(null);
      setQ("");
      setHits([]);
      setMotif("");
      await load();
    } finally {
      setBusy(false);
    }
  }

  async function sortir(id: string) {
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/sante/passages", {
        method: "PATCH",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, action: "close" }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Clôture impossible.");
        return;
      }
      setOkMsg("Passage clos — l’élève n’est plus signalé à l’infirmerie.");
      await load();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Arrivée à l’infirmerie
        </h2>
        <p className="mt-1 text-sm text-slate-600">
          Le motif court n’est pas le dossier médical. La vie scolaire voit seulement « à
          l’infirmerie ».
        </p>
        <div className="mt-4 space-y-3">
          <label className="block text-sm font-semibold text-slate-800">
            Élève
            <input
              value={selected ? `${selected.prenom} ${selected.nom}` : q}
              onChange={(e) => {
                setSelected(null);
                setQ(e.target.value);
              }}
              placeholder="Nom ou prénom (2 lettres min.)"
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
                    <span className="font-semibold text-slate-900">
                      {e.prenom} {e.nom}
                    </span>
                    <span className="text-xs text-slate-500">{e.classe || "—"}</span>
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
          <label className="block text-sm font-semibold text-slate-800">
            Motif court (optionnel)
            <input
              value={motif}
              onChange={(e) => setMotif(e.target.value)}
              placeholder="Ex. maux de tête"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <button
            type="button"
            disabled={busy || !selected}
            onClick={() => void arriver()}
            className="rounded-xl bg-rose-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer l’arrivée
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
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Présents à l’infirmerie
        </h2>
        {passages.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucun élève en ce moment.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {passages.map((p) => (
              <li
                key={p.id}
                className="flex flex-col gap-2 py-3 sm:flex-row sm:items-center sm:justify-between"
              >
                <div>
                  <p className="font-semibold text-slate-900">
                    {p.elevePrenom} {p.eleveNom}
                    {p.eleveClasse ? (
                      <span className="ml-2 text-xs font-medium text-slate-500">
                        {p.eleveClasse}
                      </span>
                    ) : null}
                  </p>
                  <p className="text-xs text-slate-500">
                    Arrivée {formatHeure(p.arrivee)}
                    {p.motifCourt ? ` · ${p.motifCourt}` : ""}
                  </p>
                </div>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void sortir(p.id)}
                  className="rounded-xl border border-slate-300 px-3 py-1.5 text-xs font-bold text-slate-800 hover:bg-slate-50 disabled:opacity-50"
                >
                  Sortie infirmerie
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
