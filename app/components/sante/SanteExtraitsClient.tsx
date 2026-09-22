"use client";

import { useCallback, useEffect, useState } from "react";
import {
  SANTE_EXTRAIT_PORTEES,
  SANTE_EXTRAIT_PORTEE_LABELS,
  type SanteExtraitPortee,
  type SanteExtraitRow,
} from "@/app/lib/sante-extraits-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };
type PorteeOpt = { id: SanteExtraitPortee; label: string };

export default function SanteExtraitsClient() {
  const [extraits, setExtraits] = useState<SanteExtraitRow[]>([]);
  const [portees, setPortees] = useState<PorteeOpt[]>(
    SANTE_EXTRAIT_PORTEES.map((p) => ({ id: p, label: SANTE_EXTRAIT_PORTEE_LABELS[p] })),
  );
  const [filtre, setFiltre] = useState<string>("");
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [portee, setPortee] = useState<SanteExtraitPortee>("cantine");
  const [libelle, setLibelle] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const params = new URLSearchParams();
    if (filtre) params.set("portee", filtre);
    const res = await fetch(`/api/sante/extraits?${params}`, { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      extraits?: SanteExtraitRow[];
      portees?: PorteeOpt[];
      error?: string;
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setExtraits(data.extraits ?? []);
    if (data.portees?.length) setPortees(data.portees);
  }, [filtre]);

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
        const res = await fetch(`/api/sante/extraits?q=${encodeURIComponent(q.trim())}`, {
          credentials: "include",
        });
        const data = (await res.json().catch(() => ({}))) as { eleves?: EleveLite[] };
        if (res.ok) setHits(data.eleves ?? []);
      })();
    }, 220);
    return () => clearTimeout(t);
  }, [q]);

  async function creer() {
    if (!selected) {
      setError("Choisissez un élève.");
      return;
    }
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/sante/extraits", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eleveId: selected.id,
          portee,
          libelle,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Création impossible.");
        return;
      }
      setOkMsg(
        `Extrait ${SANTE_EXTRAIT_PORTEE_LABELS[portee]} enregistré pour ${selected.prenom} ${selected.nom}.`,
      );
      setSelected(null);
      setQ("");
      setLibelle("");
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
      const res = await fetch("/api/sante/extraits", {
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
      setOkMsg("Extrait désactivé (conservé, plus diffusé).");
      await load();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Nouvel extrait
        </h2>
        <p className="mt-1 text-sm text-slate-600">
          Ce que la cantine, l’EPS, un voyage ou l’internat a le droit de savoir. Pas le dossier
          médical. Le PAI reste un document.
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
          <label className="block text-sm font-semibold text-slate-800">
            Portée
            <select
              value={portee}
              onChange={(e) => setPortee(e.target.value as SanteExtraitPortee)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            >
              {portees.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Libellé
            <input
              value={libelle}
              onChange={(e) => setLibelle(e.target.value)}
              placeholder="Ex. allergie arachides — pas de traces"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <button
            type="button"
            disabled={busy || !selected || !libelle.trim()}
            onClick={() => void creer()}
            className="rounded-xl bg-rose-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer l’extrait
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
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
            Extraits actifs
          </h2>
          <select
            value={filtre}
            onChange={(e) => setFiltre(e.target.value)}
            className="rounded-xl border border-slate-200 px-3 py-1.5 text-xs"
          >
            <option value="">Toutes portées</option>
            {portees.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </div>
        {extraits.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucun extrait actif.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {extraits.map((x) => (
              <li
                key={x.id}
                className="flex flex-col gap-2 py-3 sm:flex-row sm:items-center sm:justify-between"
              >
                <div>
                  <p className="font-semibold text-slate-900">
                    {x.elevePrenom} {x.eleveNom}
                    <span className="ml-2 rounded-full border border-rose-200 bg-rose-50 px-2 py-0.5 text-[10px] font-bold uppercase text-rose-900">
                      {SANTE_EXTRAIT_PORTEE_LABELS[x.portee]}
                    </span>
                  </p>
                  <p className="text-sm text-slate-600">{x.libelle}</p>
                </div>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void desactiver(x.id)}
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
