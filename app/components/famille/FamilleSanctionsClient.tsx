"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Sanction = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  typeLibelle: string;
  dateSanction: string;
  motif: string | null;
  createdByNom: string | null;
};

type Enfant = { id: string; nom: string; prenom: string; classe: string | null };

export default function FamilleSanctionsClient() {
  const [sanctions, setSanctions] = useState<Sanction[]>([]);
  const [enfants, setEnfants] = useState<Enfant[]>([]);
  const [enfantId, setEnfantId] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const qs = new URLSearchParams();
      if (enfantId) qs.set("enfant", enfantId);
      const res = await fetch(`/api/famille/sanctions?${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setSanctions(data.sanctions || []);
      setEnfants(data.enfants || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, [enfantId]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="min-h-screen bg-slate-50 text-slate-900">
      <div className="mx-auto max-w-3xl px-4 py-8">
        <header className="mb-6">
          <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Portail familles</p>
          <h1 className="text-2xl font-black">Sanctions</h1>
          <p className="text-sm text-slate-600 mt-1">
            Sanctions actives de votre enfant. L’accusé de lecture se fait aussi via le{" "}
            <Link href="/famille/carnet" className="font-bold text-indigo-700 underline">
              carnet de liaison
            </Link>
            .
          </p>
          <FamilleNav enfantId={enfantId || null} />
        </header>

        {error ? <p className="mb-3 text-sm font-semibold text-rose-700">{error}</p> : null}

        {enfants.length > 1 ? (
          <label className="mb-4 block text-sm">
            Enfant
            <select
              className="mt-1 w-full max-w-xs rounded-lg border border-slate-200 px-3 py-2"
              value={enfantId}
              onChange={(e) => setEnfantId(e.target.value)}
            >
              <option value="">Tous</option>
              {enfants.map((e) => (
                <option key={e.id} value={e.id}>
                  {e.prenom} {e.nom}
                  {e.classe ? ` (${e.classe})` : ""}
                </option>
              ))}
            </select>
          </label>
        ) : null}

        {busy && !sanctions.length ? (
          <p className="text-sm text-slate-500">Chargement…</p>
        ) : sanctions.length === 0 ? (
          <p className="text-sm text-slate-500 rounded-2xl border border-slate-200 bg-white p-4">
            Aucune sanction active.
          </p>
        ) : (
          <ul className="space-y-3">
            {sanctions.map((s) => (
              <li key={s.id} className="rounded-2xl border border-slate-200 bg-white p-4 text-sm">
                <div className="font-bold text-slate-900">
                  {s.typeLibelle} · {s.dateSanction}
                </div>
                <p className="text-xs text-slate-500 mt-0.5">
                  {s.elevePrenom} {s.eleveNom}
                  {s.eleveClasse ? ` (${s.eleveClasse})` : ""}
                  {s.createdByNom ? ` · ${s.createdByNom}` : ""}
                </p>
                {s.motif ? (
                  <p className="mt-2 whitespace-pre-wrap text-slate-800">{s.motif}</p>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
