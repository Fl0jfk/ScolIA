"use client";

import { useCallback, useEffect, useState } from "react";
import {
  PASSAGE_LIEU_LABELS,
  PASSAGE_SENS_LABELS,
  type PassageLieu,
  type PassageRow,
  type PassageSens,
} from "@/app/lib/passages-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

function formatHeure(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  } catch {
    return iso;
  }
}

type Props = {
  /** Lieu par défaut / filtré pour cet écran. */
  lieu: PassageLieu;
};

export default function PassagesClient({ lieu }: Props) {
  const [passages, setPassages] = useState<PassageRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [sens, setSens] = useState<PassageSens>(lieu === "self" ? "entree" : "entree");
  const [alertes, setAlertes] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const res = await fetch(`/api/passages?lieu=${encodeURIComponent(lieu)}`, {
      credentials: "include",
    });
    const data = (await res.json().catch(() => ({}))) as {
      passages?: PassageRow[];
      error?: string;
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setPassages(data.passages ?? []);
  }, [lieu]);

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
        const res = await fetch(`/api/passages?q=${encodeURIComponent(q.trim())}`, {
          credentials: "include",
        });
        const data = (await res.json().catch(() => ({}))) as { eleves?: EleveLite[] };
        if (res.ok) setHits(data.eleves ?? []);
      })();
    }, 220);
    return () => clearTimeout(t);
  }, [q, selected]);

  useEffect(() => {
    if (!selected || lieu !== "self") {
      setAlertes([]);
      return;
    }
    void (async () => {
      const res = await fetch(
        `/api/passages?eleveId=${encodeURIComponent(selected.id)}&alertes=cantine`,
        { credentials: "include" },
      );
      const data = (await res.json().catch(() => ({}))) as { alertes?: string[] };
      if (res.ok) setAlertes(data.alertes ?? []);
    })();
  }, [selected, lieu]);

  async function enregistrer() {
    if (!selected) {
      setError("Choisissez un élève.");
      return;
    }
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/passages", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eleveId: selected.id,
          sens,
          lieu,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        alertesCantine?: string[];
        passage?: PassageRow;
      };
      if (!res.ok) {
        setError(data.error || "Enregistrement impossible.");
        return;
      }
      const who = `${selected.prenom} ${selected.nom}`;
      const lieuLabel = PASSAGE_LIEU_LABELS[lieu];
      const sensLabel = PASSAGE_SENS_LABELS[sens];
      let msg = `${who} — ${sensLabel} ${lieuLabel}.`;
      if (data.alertesCantine && data.alertesCantine.length > 0) {
        msg += ` Alerte cantine : ${data.alertesCantine.join(" · ")}`;
      }
      setOkMsg(msg);
      setSelected(null);
      setQ("");
      setAlertes([]);
      await load();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          {lieu === "self" ? "Repas pris (self)" : "Passage portail"}
        </h2>
        <p className="mt-1 text-sm text-slate-600">
          {lieu === "self"
            ? "Le repas pris est un passage. Le régime / la grille = le droit, pas le pris."
            : "Entrée ou sortie de l’établissement. « Où est X » lit la dernière sortie portail."}
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
          {alertes.length > 0 ? (
            <div className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950">
              <p className="font-bold">Extrait cantine</p>
              <ul className="mt-1 list-disc pl-5">
                {alertes.map((a) => (
                  <li key={a}>{a}</li>
                ))}
              </ul>
            </div>
          ) : null}
          {lieu === "portail" ? (
            <label className="block text-sm font-semibold text-slate-800">
              Sens
              <select
                value={sens}
                onChange={(e) => setSens(e.target.value as PassageSens)}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              >
                <option value="entree">Entrée</option>
                <option value="sortie">Sortie</option>
              </select>
            </label>
          ) : (
            <p className="text-xs text-slate-500">Sens : entrée (repas enregistré).</p>
          )}
          <button
            type="button"
            disabled={busy || !selected}
            onClick={() => void enregistrer()}
            className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer
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
          Aujourd’hui — {PASSAGE_LIEU_LABELS[lieu]}
        </h2>
        {passages.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucun passage enregistré aujourd’hui.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {passages.map((p) => (
              <li key={p.id} className="flex items-center justify-between py-2 text-sm">
                <span className="font-semibold text-slate-900">
                  {p.elevePrenom && p.eleveNom
                    ? `${p.elevePrenom} ${p.eleveNom}`
                    : p.inviteNom || "—"}
                  {p.eleveClasse ? (
                    <span className="ml-2 text-xs font-medium text-slate-500">{p.eleveClasse}</span>
                  ) : null}
                </span>
                <span className="text-xs text-slate-500">
                  {formatHeure(p.horodatage)} · {PASSAGE_SENS_LABELS[p.sens]}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
