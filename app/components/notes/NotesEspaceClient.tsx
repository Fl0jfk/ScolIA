"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type Matiere = { id: string; code: string; libelle: string; actif: boolean };
type Periode = { id: string; code: string; libelle: string; ordre: number; statut: string };
type TypeDevoir = { id: string; code: string; libelle: string };

export default function NotesEspaceClient() {
  const [matieres, setMatieres] = useState<Matiere[]>([]);
  const [periodes, setPeriodes] = useState<Periode[]>([]);
  const [typesDevoir, setTypesDevoir] = useState<TypeDevoir[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [matiereForm, setMatiereForm] = useState({ code: "", libelle: "" });
  const [periodeForm, setPeriodeForm] = useState({ code: "", libelle: "", ordre: "1" });
  const [anneeLabel, setAnneeLabel] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/notes/config", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setMatieres(data.matieres || []);
      setPeriodes(data.periodes || []);
      setTypesDevoir(data.typesDevoir || []);
      if (data.anneeCourante?.label) setAnneeLabel(String(data.anneeCourante.label));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const post = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/notes/config", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      setMessage(data.seeded === false ? "Référentiels déjà présents." : "Enregistré.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Administratif · Notes"
        title="Notes & bulletins — configuration"
        description={
          anneeLabel
            ? `Année ${anneeLabel} — référentiels, saisie classe/groupe, LSU, bulletins PDF.`
            : "Référentiels, saisie par classe ou groupe, compétences LSU, bulletins PDF — même registre que le dossier élève."
        }
        actions={
          <div className="flex flex-wrap gap-3 text-sm font-bold">
            <Link href="/parametres?tab=annees" className="text-slate-600 hover:underline">
              Année scolaire
            </Link>
            <Link href="/administratif" className="text-indigo-600 hover:underline">
              ← Administratif
            </Link>
          </div>
        }
      />

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5 mb-6">
        <Link
          href="/notes/saisie"
          className="rounded-2xl border border-indigo-200 bg-indigo-50 p-4 hover:bg-indigo-100"
        >
          <p className="font-bold text-indigo-900">Saisie</p>
          <p className="text-xs text-indigo-800 mt-1">Devoirs, notes, moyennes</p>
        </Link>
        <Link
          href="/notes/competences"
          className="rounded-2xl border border-slate-200 bg-white p-4 hover:bg-slate-50"
        >
          <p className="font-bold text-slate-900">Compétences LSU</p>
          <p className="text-xs text-slate-600 mt-1">Grille maîtrise collège</p>
        </Link>
        <Link
          href="/notes/bulletins"
          className="rounded-2xl border border-slate-200 bg-white p-4 hover:bg-slate-50"
        >
          <p className="font-bold text-slate-900">Bulletins PDF</p>
          <p className="text-xs text-slate-600 mt-1">Édition classe ou groupe</p>
        </Link>
        <Link
          href="/groupes-pedagogiques"
          className="rounded-2xl border border-slate-200 bg-white p-4 hover:bg-slate-50"
        >
          <p className="font-bold text-slate-900">Groupes</p>
          <p className="text-xs text-slate-600 mt-1">LV2, options, demi-groupes</p>
        </Link>
        <Link
          href="/toolbox/repartition-classes"
          className="rounded-2xl border border-emerald-200 bg-emerald-50 p-4 hover:bg-emerald-100"
        >
          <p className="font-bold text-emerald-900">Composition classes</p>
          <p className="text-xs text-emerald-800 mt-1">Rentrée → registre élèves</p>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3 mb-6">
        <button
          type="button"
          disabled={busy}
          onClick={() => void post({ action: "seedDefaults" })}
          className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
        >
          Initialiser collège (matières + trimestres)
        </button>
      </div>

      {message && <p className="mb-3 text-sm text-emerald-700 font-semibold">{message}</p>}
      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}

      <div className="grid gap-6 md:grid-cols-2">
        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="font-black text-slate-900">Matières ({matieres.length})</h2>
          <div className="flex gap-2">
            <input
              className="border rounded-xl px-3 py-2 text-sm w-24"
              placeholder="Code"
              value={matiereForm.code}
              onChange={(e) => setMatiereForm({ ...matiereForm, code: e.target.value })}
            />
            <input
              className="border rounded-xl px-3 py-2 text-sm flex-1"
              placeholder="Libellé"
              value={matiereForm.libelle}
              onChange={(e) => setMatiereForm({ ...matiereForm, libelle: e.target.value })}
            />
            <button
              type="button"
              disabled={busy}
              className="rounded-xl bg-slate-900 text-white px-3 py-2 text-sm font-bold"
              onClick={() =>
                void post({ action: "upsertMatiere", ...matiereForm }).then(() =>
                  setMatiereForm({ code: "", libelle: "" }),
                )
              }
            >
              +
            </button>
          </div>
          <ul className="text-sm divide-y max-h-64 overflow-y-auto">
            {matieres.map((m) => (
              <li key={m.id} className="py-2 flex justify-between gap-2">
                <span>
                  <span className="font-mono text-xs text-slate-500">{m.code}</span>{" "}
                  {m.libelle}
                </span>
                {!m.actif && <span className="text-xs text-slate-400">inactif</span>}
              </li>
            ))}
          </ul>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="font-black text-slate-900">Périodes ({periodes.length})</h2>
          <div className="flex gap-2 flex-wrap">
            <input
              className="border rounded-xl px-3 py-2 text-sm w-20"
              placeholder="T1"
              value={periodeForm.code}
              onChange={(e) => setPeriodeForm({ ...periodeForm, code: e.target.value })}
            />
            <input
              className="border rounded-xl px-3 py-2 text-sm flex-1 min-w-[8rem]"
              placeholder="Libellé"
              value={periodeForm.libelle}
              onChange={(e) => setPeriodeForm({ ...periodeForm, libelle: e.target.value })}
            />
            <button
              type="button"
              disabled={busy}
              className="rounded-xl bg-slate-900 text-white px-3 py-2 text-sm font-bold"
              onClick={() =>
                void post({
                  action: "upsertPeriode",
                  code: periodeForm.code,
                  libelle: periodeForm.libelle,
                  ordre: Number(periodeForm.ordre) || 1,
                }).then(() => setPeriodeForm({ code: "", libelle: "", ordre: "1" }))
              }
            >
              +
            </button>
          </div>
          <ul className="text-sm divide-y">
            {periodes.map((p) => (
              <li key={p.id} className="py-2 flex justify-between">
                <span>
                  <span className="font-mono text-xs text-slate-500">{p.code}</span> {p.libelle}
                </span>
                <span className="text-xs text-slate-500">{p.statut}</span>
              </li>
            ))}
          </ul>

          <h3 className="font-bold text-slate-800 pt-2">Types de devoirs</h3>
          <ul className="text-sm flex flex-wrap gap-2">
            {typesDevoir.map((t) => (
              <li key={t.id} className="rounded-lg bg-slate-100 px-2 py-1 text-xs font-semibold">
                {t.code} — {t.libelle}
              </li>
            ))}
            {!typesDevoir.length && (
              <li className="text-slate-500 text-xs">Initialisez pour créer DS / DM / Interro / Oral.</li>
            )}
          </ul>
        </section>
      </div>
    </ModulePageShell>
  );
}
