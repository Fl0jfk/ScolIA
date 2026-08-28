"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";

export default function StagePreconventionStartPage() {
  const router = useRouter();
  const [ine, setIne] = useState("");
  const [dateNaissance, setDateNaissance] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<{
    firstName: string;
    lastName: string;
    className: string;
  } | null>(null);

  async function start(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setPreview(null);
    try {
      const res = await fetch("/api/stages/public/preconvention", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ine, dateNaissance }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      if (data.studentPreview) setPreview(data.studentPreview);
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
          Identifiez-vous avec les informations figurant sur le bulletin scolaire ou dans Pronote.
          Aucune liste d&apos;élèves n&apos;est affichée : seul l&apos;établissement vérifie votre
          dossier en interne.
        </p>

        {error && <p className="mt-4 text-sm text-rose-700">{error}</p>}
        {preview && (
          <p className="mt-4 rounded-lg bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-800">
            Bienvenue {preview.firstName} {preview.lastName} ({preview.className}) — redirection…
          </p>
        )}

        <form onSubmit={(e) => void start(e)} className="mt-6 space-y-4 text-sm">
          <label className="block">
            <span className="text-xs font-semibold text-stone-600">
              Identifiant national élève (INE) *
            </span>
            <input
              className="mt-1 w-full rounded-lg border px-3 py-2 font-mono uppercase"
              placeholder="ex. 180123456AB"
              value={ine}
              onChange={(e) => setIne(e.target.value.toUpperCase())}
              autoComplete="off"
              required
            />
            <span className="mt-1 block text-xs text-stone-500">
              Code à 11 caractères sur le bulletin — connu des parents, jamais publié en liste.
            </span>
          </label>
          <label className="block">
            <span className="text-xs font-semibold text-stone-600">Date de naissance *</span>
            <input
              type="date"
              className="mt-1 w-full rounded-lg border px-3 py-2"
              value={dateNaissance}
              onChange={(e) => setDateNaissance(e.target.value)}
              required
            />
          </label>
          <button
            type="submit"
            disabled={busy}
            className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
          >
            {busy ? "Vérification…" : "Accéder à ma préconvention →"}
          </button>
        </form>

        <div className="mt-6 space-y-2 text-xs text-stone-500">
          <p>
            Vous avez reçu un lien personnel du professeur principal ou du secrétariat ? Ouvrez-le
            directement — pas besoin de saisir l&apos;INE ici.
          </p>
          <p className="font-mono text-stone-600">/stages/eleve?token=…</p>
        </div>
      </div>
    </main>
  );
}
