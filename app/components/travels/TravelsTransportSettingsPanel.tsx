"use client";

import { useCallback, useEffect, useState } from "react";

type TravelsCfg = {
  transportProviders: { name: string; email: string }[];
  pdfFooterText?: string;
};

/** Paramétrage transporteurs — réservé admin global, dans le module Sorties. */
export default function TravelsTransportSettingsPanel() {
  const [cfg, setCfg] = useState<TravelsCfg>({ transportProviders: [] });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/settings/travels", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setCfg(
        j.travels || {
          transportProviders: [],
          pdfFooterText: "",
        },
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const save = async () => {
    setSaving(true);
    setMsg(null);
    setError(null);
    try {
      const res = await fetch("/api/settings/travels", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(cfg),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      setMsg("Paramètres sorties enregistrés.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement…</p>;
  }

  return (
    <div className="rounded-2xl border border-slate-200 bg-white p-6 space-y-4 max-w-2xl">
      <div>
        <h2 className="text-lg font-black text-slate-900">Paramétrage transporteurs</h2>
        <p className="mt-1 text-sm text-slate-600">
          Liste des transporteurs sollicités pour les devis bus, et pied de page des PDF.
        </p>
      </div>
      {error && <p className="text-sm text-rose-700">{error}</p>}
      {msg && <p className="text-sm text-emerald-700">{msg}</p>}
      <label className="block text-sm font-bold text-slate-800">
        Texte pied de page PDF
        <input
          className="mt-1 w-full rounded-xl border border-slate-200 p-3 text-sm"
          value={cfg.pdfFooterText || ""}
          onChange={(e) => setCfg({ ...cfg, pdfFooterText: e.target.value })}
        />
      </label>
      <p className="text-sm font-bold text-slate-800">Transporteurs</p>
      {cfg.transportProviders.map((p, idx) => (
        <div key={idx} className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          <input
            className="rounded-lg border border-slate-200 p-2 text-sm"
            placeholder="Nom"
            value={p.name}
            onChange={(e) => {
              const copy = [...cfg.transportProviders];
              copy[idx] = { ...copy[idx], name: e.target.value };
              setCfg({ ...cfg, transportProviders: copy });
            }}
          />
          <input
            className="rounded-lg border border-slate-200 p-2 text-sm"
            placeholder="E-mail"
            type="email"
            value={p.email}
            onChange={(e) => {
              const copy = [...cfg.transportProviders];
              copy[idx] = { ...copy[idx], email: e.target.value };
              setCfg({ ...cfg, transportProviders: copy });
            }}
          />
        </div>
      ))}
      <button
        type="button"
        className="text-sm font-bold text-indigo-600"
        onClick={() =>
          setCfg({
            ...cfg,
            transportProviders: [...cfg.transportProviders, { name: "", email: "" }],
          })
        }
      >
        + Transporteur
      </button>
      <div>
        <button
          type="button"
          disabled={saving}
          onClick={() => void save()}
          className="rounded-xl bg-indigo-600 px-6 py-2.5 text-sm font-bold text-white disabled:opacity-50"
        >
          {saving ? "…" : "Enregistrer"}
        </button>
      </div>
    </div>
  );
}
