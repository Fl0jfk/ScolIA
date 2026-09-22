"use client";

import { useCallback, useEffect, useState } from "react";
import type { SanteInaptitudeEpsRow } from "@/app/lib/sante-inaptitudes-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

export default function SanteInaptitudesClient() {
  const [rows, setRows] = useState<SanteInaptitudeEpsRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [dateDebut, setDateDebut] = useState(todayIso);
  const [dateFin, setDateFin] = useState("");
  const [motif, setMotif] = useState("");
  const [libelle, setLibelle] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const res = await fetch("/api/sante/inaptitudes", { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      inaptitudes?: SanteInaptitudeEpsRow[];
      error?: string;
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setRows(data.inaptitudes ?? []);
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
        const res = await fetch(`/api/sante/inaptitudes?q=${encodeURIComponent(q.trim())}`, {
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
      const res = await fetch("/api/sante/inaptitudes", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eleveId: selected.id,
          dateDebut,
          dateFin: dateFin || null,
          motif,
          libelleExtrait: libelle,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Création impossible.");
        return;
      }
      setOkMsg(
        `Inaptitude EPS enregistrée pour ${selected.prenom} ${selected.nom}. Extrait EPS diffusé.`,
      );
      setMotif("");
      setLibelle("");
      setDateFin("");
      setDateDebut(todayIso());
      await load();
    } finally {
      setBusy(false);
    }
  }

  async function desactiver(id: string) {
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/sante/inaptitudes", {
        method: "PATCH",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, action: "desactiver" }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Désactivation impossible.");
        return;
      }
      setOkMsg("Inaptitude et extrait EPS désactivés (conservés).");
      await load();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Nouvelle inaptitude
        </h2>
        <p className="mt-1 text-sm text-slate-600">
          Certificat daté → extrait EPS. Le motif reste à l’infirmerie ; le libellé circule vers
          l’EPS.
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
            <label className="block text-sm font-semibold text-slate-800">
              Début
              <input
                type="date"
                value={dateDebut}
                onChange={(e) => setDateDebut(e.target.value)}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              />
            </label>
            <label className="block text-sm font-semibold text-slate-800">
              Fin (optionnel)
              <input
                type="date"
                value={dateFin}
                onChange={(e) => setDateFin(e.target.value)}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              />
            </label>
          </div>
          <label className="block text-sm font-semibold text-slate-800">
            Motif (infirmerie)
            <input
              value={motif}
              onChange={(e) => setMotif(e.target.value)}
              placeholder="Ex. entorse cheville — certificat Dr X"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Libellé pour l’EPS
            <input
              value={libelle}
              onChange={(e) => setLibelle(e.target.value)}
              placeholder="Ex. inapte EPS — pas de course"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <button
            type="button"
            disabled={busy || !selected || !libelle.trim()}
            onClick={() => void creer()}
            className="rounded-xl bg-rose-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer + diffuser extrait EPS
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
          Inaptitudes actives
        </h2>
        {rows.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucune inaptitude active.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {rows.map((r) => (
              <li
                key={r.id}
                className="flex flex-col gap-2 py-3 sm:flex-row sm:items-center sm:justify-between"
              >
                <div>
                  <p className="font-semibold text-slate-900">
                    {r.elevePrenom} {r.eleveNom}
                    <span className="ml-2 rounded-full border border-sky-200 bg-sky-50 px-2 py-0.5 text-[10px] font-bold uppercase text-sky-900">
                      EPS
                    </span>
                  </p>
                  <p className="text-sm text-slate-700">{r.libelleExtrait}</p>
                  <p className="text-xs text-slate-500">
                    {r.dateDebut}
                    {r.dateFin ? ` → ${r.dateFin}` : " → …"}
                    {r.motif ? ` · ${r.motif}` : ""}
                  </p>
                </div>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void desactiver(r.id)}
                  className="rounded-xl border border-slate-300 px-3 py-1.5 text-xs font-bold text-slate-800 hover:bg-slate-50 disabled:opacity-50"
                >
                  Désactiver
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
