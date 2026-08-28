"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";

const LEVELS = ["6e", "5e", "4e", "3e", "2nde", "1re", "Tle"];

export default function StagePreconventionStartPage() {
  const router = useRouter();
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [className, setClassName] = useState("");
  const [level, setLevel] = useState("3e");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function start(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/public/preconvention", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ firstName, lastName, className, level }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      router.push(data.studentLink);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="min-h-screen bg-[#f6f8f5] px-4 py-10">
      <div className="mx-auto max-w-lg rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h1 className="text-2xl font-black text-[#1F3D2B]">Préconvention de stage</h1>
        <p className="mt-2 text-sm text-stone-600">
          Commencez votre dossier en ligne : entreprise, horaires, contacts. Une fois validé par
          l&apos;administration, la convention sera envoyée aux signataires (famille, tuteur,
          professeur principal, direction).
        </p>

        {error && <p className="mt-4 text-sm text-rose-700">{error}</p>}

        <form onSubmit={(e) => void start(e)} className="mt-6 space-y-4 text-sm">
          <div className="grid grid-cols-2 gap-2">
            <input
              className="rounded-lg border px-3 py-2"
              placeholder="Prénom *"
              value={firstName}
              onChange={(e) => setFirstName(e.target.value)}
              required
            />
            <input
              className="rounded-lg border px-3 py-2"
              placeholder="Nom *"
              value={lastName}
              onChange={(e) => setLastName(e.target.value)}
              required
            />
          </div>
          <input
            className="w-full rounded-lg border px-3 py-2"
            placeholder="Classe * (ex. 3e2)"
            value={className}
            onChange={(e) => setClassName(e.target.value)}
            required
          />
          <label className="block">
            <span className="text-xs font-semibold text-stone-600">Niveau</span>
            <select
              className="mt-1 w-full rounded-lg border px-3 py-2"
              value={level}
              onChange={(e) => setLevel(e.target.value)}
            >
              {LEVELS.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </select>
          </label>
          <button
            type="submit"
            disabled={busy}
            className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
          >
            {busy ? "Création…" : "Commencer ma préconvention →"}
          </button>
        </form>

        <p className="mt-6 text-xs text-stone-500">
          Vous avez déjà un lien ? Ouvrez directement{" "}
          <span className="font-mono text-stone-700">/stages/eleve?token=…</span>
        </p>
      </div>
    </main>
  );
}
