"use client";

import { useState } from "react";
import Link from "next/link";
import RequireOrgAdmin from "@/app/components/RequireOrgAdmin";

type Pair = { giver: string; receiver: string };

export default function SecretSantaPage() {
  const [namesText, setNamesText] = useState("");
  const [pairs, setPairs] = useState<Pair[] | null>(null);
  const [budget, setBudget] = useState("");
  const [title, setTitle] = useState("Secret Santa");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [revealed, setRevealed] = useState(false);

  async function draw() {
    setBusy(true);
    setError(null);
    setPairs(null);
    setRevealed(false);
    try {
      const names = namesText.split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
      const res = await fetch("/api/toolbox/secret-santa/draw", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ names }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setPairs(j.pairs);
      setBudget(j.budgetHint || "");
      setTitle(j.title || "Secret Santa");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <RequireOrgAdmin>
      <main className="mx-auto max-w-2xl px-4 py-10">
        <Link
          href="/etablissement/evenements?tab=secret-santa"
          className="text-sm font-bold text-slate-500 hover:text-slate-800"
        >
          ← Événements
        </Link>
        <h1 className="mt-4 text-3xl font-black text-red-700">{title}</h1>
        <p className="mt-2 text-sm text-slate-600">
          Tirage anonyme pour l&apos;équipe. Les résultats ne sont affichés qu&apos;ici — ne les
          partagez pas publiquement.
        </p>

        {error && (
          <p className="mt-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
            {error}
          </p>
        )}

        {!pairs ? (
          <div className="mt-8 space-y-4">
            <label className="block">
              <span className="text-xs font-bold uppercase text-slate-500">
                Participants (un par ligne)
              </span>
              <textarea
                className="mt-2 w-full min-h-[200px] rounded-2xl border border-slate-200 px-4 py-3 font-mono text-sm"
                value={namesText}
                onChange={(e) => setNamesText(e.target.value)}
                placeholder="Marie Dupont&#10;Jean Martin&#10;…"
              />
            </label>
            <button
              type="button"
              onClick={() => void draw()}
              disabled={busy}
              className="rounded-xl bg-red-600 px-6 py-3 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Tirage…" : "Lancer le tirage"}
            </button>
          </div>
        ) : (
          <div className="mt-8">
            {budget && <p className="text-sm text-slate-600 mb-4">Budget indicatif : {budget}</p>}
            {!revealed ? (
              <button
                type="button"
                onClick={() => setRevealed(true)}
                className="rounded-xl bg-slate-900 px-6 py-3 text-sm font-bold text-white"
              >
                Afficher les résultats (admin uniquement)
              </button>
            ) : (
              <ul className="space-y-2 rounded-2xl border border-slate-200 bg-white p-6">
                {pairs.map((p) => (
                  <li key={p.giver} className="text-sm">
                    <span className="font-bold">{p.giver}</span>
                    <span className="text-slate-500"> offre un cadeau à </span>
                    <span className="font-bold text-red-700">{p.receiver}</span>
                  </li>
                ))}
              </ul>
            )}
            <button
              type="button"
              onClick={() => {
                setPairs(null);
                setRevealed(false);
              }}
              className="mt-4 text-sm font-bold text-slate-500 underline"
            >
              Nouveau tirage
            </button>
          </div>
        )}
      </main>
    </RequireOrgAdmin>
  );
}
