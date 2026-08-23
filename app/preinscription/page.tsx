"use client";

import { useState } from "react";

export default function PreinscriptionPublicPage() {
  const [nom, setNom] = useState("");
  const [prenom, setPrenom] = useState("");
  const [dateNaissance, setDateNaissance] = useState("");
  const [lieuNaissance, setLieuNaissance] = useState("");
  const [siteId, setSiteId] = useState("college");
  const [niveauVise, setNiveauVise] = useState("");
  const [demiPension, setDemiPension] = useState(false);
  const [etablissementPrecedent, setEtablissementPrecedent] = useState("");
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/public/preinscriptions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          nom,
          prenom,
          dateNaissance: dateNaissance || undefined,
          lieuNaissance: lieuNaissance || undefined,
          siteId,
          niveauVise: niveauVise || undefined,
          demiPension,
          etablissementPrecedent: etablissementPrecedent || undefined,
        }),
      });
      if (!res.ok) {
        const j = (await res.json().catch(() => ({}))) as { error?: string };
        throw new Error(j.error || "Envoi impossible");
      }
      setDone(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  if (done) {
    return (
      <main className="min-h-screen flex items-center justify-center bg-slate-50 px-4">
        <div className="max-w-md rounded-3xl border border-emerald-200 bg-white p-8 text-center shadow-sm">
          <h1 className="text-xl font-bold text-slate-900">Demande envoyée</h1>
          <p className="mt-2 text-sm text-slate-600">
            Votre préinscription a bien été reçue. L’établissement vous recontactera.
          </p>
        </div>
      </main>
    );
  }

  return (
    <main className="min-h-screen bg-slate-50 px-4 py-12">
      <form
        onSubmit={submit}
        className="mx-auto max-w-lg space-y-4 rounded-3xl border border-slate-200 bg-white p-8 shadow-sm"
      >
        <h1 className="text-2xl font-bold text-slate-900">Préinscription</h1>
        <p className="text-sm text-slate-500">Formulaire public — groupe scolaire</p>
        {error ? <p className="text-sm text-red-600">{error}</p> : null}
        <label className="block text-sm">
          <span className="font-medium">Nom</span>
          <input required value={nom} onChange={(e) => setNom(e.target.value)} className="mt-1 w-full rounded-xl border px-3 py-2" />
        </label>
        <label className="block text-sm">
          <span className="font-medium">Prénom</span>
          <input required value={prenom} onChange={(e) => setPrenom(e.target.value)} className="mt-1 w-full rounded-xl border px-3 py-2" />
        </label>
        <label className="block text-sm">
          <span className="font-medium">Date de naissance</span>
          <input type="date" value={dateNaissance} onChange={(e) => setDateNaissance(e.target.value)} className="mt-1 w-full rounded-xl border px-3 py-2" />
        </label>
        <label className="block text-sm">
          <span className="font-medium">Lieu de naissance</span>
          <input value={lieuNaissance} onChange={(e) => setLieuNaissance(e.target.value)} className="mt-1 w-full rounded-xl border px-3 py-2" />
        </label>
        <label className="block text-sm">
          <span className="font-medium">Établissement demandé</span>
          <select value={siteId} onChange={(e) => setSiteId(e.target.value)} className="mt-1 w-full rounded-xl border px-3 py-2">
            <option value="ecole">École</option>
            <option value="college">Collège</option>
            <option value="lycee">Lycée</option>
          </select>
        </label>
        <label className="block text-sm">
          <span className="font-medium">Niveau / classe visée</span>
          <input value={niveauVise} onChange={(e) => setNiveauVise(e.target.value)} placeholder="Ex. 6e, 2nde, 1ST2S…" className="mt-1 w-full rounded-xl border px-3 py-2" />
        </label>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={demiPension} onChange={(e) => setDemiPension(e.target.checked)} />
          Demi-pension
        </label>
        <label className="block text-sm">
          <span className="font-medium">Établissement précédent</span>
          <input value={etablissementPrecedent} onChange={(e) => setEtablissementPrecedent(e.target.value)} className="mt-1 w-full rounded-xl border px-3 py-2" />
        </label>
        <button
          type="submit"
          disabled={busy}
          className="w-full rounded-xl bg-slate-900 py-3 text-sm font-bold text-white disabled:opacity-50"
        >
          {busy ? "Envoi…" : "Envoyer la demande"}
        </button>
      </form>
    </main>
  );
}
