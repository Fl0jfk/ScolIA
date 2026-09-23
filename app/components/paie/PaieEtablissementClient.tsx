"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type Periode = {
  id: string;
  label: string;
  dateDebut: string;
  dateFin: string;
  statut: string;
};

type Personne = {
  id: string;
  nom: string;
  prenom: string;
  displayName?: string;
};

type Element = {
  id: string;
  nature: string;
  libelle: string;
  quantite: string | null;
  montant: string | null;
  personnelNom: string | null;
  personnelPrenom: string | null;
  personnelDisplay: string | null;
};

export default function PaieEtablissementClient() {
  const [periodes, setPeriodes] = useState<Periode[]>([]);
  const [personnel, setPersonnel] = useState<Personne[]>([]);
  const [elements, setElements] = useState<Element[]>([]);
  const [periodeId, setPeriodeId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const today = new Date().toISOString().slice(0, 10);
  const monthStart = `${today.slice(0, 8)}01`;
  const [periodeForm, setPeriodeForm] = useState({
    label: `Paie ${today.slice(0, 7)}`,
    dateDebut: monthStart,
    dateFin: today,
  });
  const [elementForm, setElementForm] = useState({
    personnelId: "",
    nature: "heure",
    libelle: "",
    quantite: "",
    montant: "",
  });

  const load = useCallback(async (pid?: string | null) => {
    setError(null);
    try {
      const qs = pid ? `?periodeId=${encodeURIComponent(pid)}` : "";
      const res = await fetch(`/api/paie${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setPeriodes(data.periodes || []);
      setPersonnel(data.personnel || []);
      setElements(data.elements || []);
      setPeriodeId(data.periodeId || null);
      setElementForm((f) =>
        f.personnelId || !(data.personnel || []).length
          ? f
          : { ...f, personnelId: data.personnel[0].id },
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const post = async (action: string, body: Record<string, unknown> = {}) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/paie", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action, ...body }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      const labels: Record<string, string> = {
        createPeriode: "Période créée.",
        createElement: "Élément de paie enregistré.",
        figerPeriode: "Période figée.",
      };
      setMessage(labels[action] || "OK.");
      const nextPid =
        action === "createPeriode" && data.periode?.id ? String(data.periode.id) : periodeId;
      await load(nextPid);
      return data;
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      return null;
    } finally {
      setBusy(false);
    }
  };

  const selected = periodes.find((p) => p.id === periodeId) || null;

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Compta & RH · Paie light"
        title="Paie établissement"
        description="Périodes et éléments (heures, absences, primes, retenues) — pas de bulletin ni DSN."
        actions={
          <Link href="/compta-rh" className="text-sm font-bold text-emerald-700 hover:underline">
            ← Comptabilité & RH
          </Link>
        }
      />

      {message && <p className="mb-3 text-sm font-semibold text-emerald-700">{message}</p>}
      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}

      <section className="mb-6 space-y-3 rounded-2xl border border-violet-200 bg-white p-5">
        <h2 className="font-black text-slate-900">Nouvelle période</h2>
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
          <input
            className="rounded-xl border px-3 py-2 text-sm sm:col-span-2"
            placeholder="Libellé"
            value={periodeForm.label}
            onChange={(e) => setPeriodeForm({ ...periodeForm, label: e.target.value })}
          />
          <input
            type="date"
            className="rounded-xl border px-3 py-2 text-sm"
            value={periodeForm.dateDebut}
            onChange={(e) => setPeriodeForm({ ...periodeForm, dateDebut: e.target.value })}
          />
          <input
            type="date"
            className="rounded-xl border px-3 py-2 text-sm"
            value={periodeForm.dateFin}
            onChange={(e) => setPeriodeForm({ ...periodeForm, dateFin: e.target.value })}
          />
          <button
            type="button"
            disabled={busy}
            onClick={() => void post("createPeriode", periodeForm)}
            className="rounded-xl bg-violet-700 px-3 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Créer période
          </button>
        </div>
        <div className="flex flex-wrap gap-2 items-center text-sm">
          <label className="text-xs text-slate-600">
            Période active{" "}
            <select
              className="rounded-lg border px-2 py-1"
              value={periodeId || ""}
              onChange={(e) => void load(e.target.value || null)}
            >
              {!periodes.length && <option value="">Aucune</option>}
              {periodes.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label} ({p.statut})
                </option>
              ))}
            </select>
          </label>
          {selected?.statut === "brouillon" ? (
            <button
              type="button"
              disabled={busy}
              className="text-xs font-bold text-violet-800"
              onClick={() => void post("figerPeriode", { periodeId: selected.id })}
            >
              Figer la période
            </button>
          ) : null}
        </div>
      </section>

      <section className="mb-6 space-y-3 rounded-2xl border border-slate-200 bg-white p-5">
        <h2 className="font-black text-slate-900">Élément de paie</h2>
        {!periodeId ? (
          <p className="text-sm text-slate-500">Créez une période pour saisir des éléments.</p>
        ) : (
          <>
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
              <select
                className="rounded-xl border px-3 py-2 text-sm sm:col-span-2"
                value={elementForm.personnelId}
                onChange={(e) => setElementForm({ ...elementForm, personnelId: e.target.value })}
              >
                {personnel.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.displayName || `${p.prenom} ${p.nom}`}
                  </option>
                ))}
              </select>
              <select
                className="rounded-xl border px-3 py-2 text-sm"
                value={elementForm.nature}
                onChange={(e) => setElementForm({ ...elementForm, nature: e.target.value })}
              >
                <option value="heure">Heure</option>
                <option value="absence">Absence</option>
                <option value="prime">Prime</option>
                <option value="retenue">Retenue</option>
              </select>
              <input
                className="rounded-xl border px-3 py-2 text-sm"
                placeholder="Quantité (h)"
                value={elementForm.quantite}
                onChange={(e) => setElementForm({ ...elementForm, quantite: e.target.value })}
              />
              <input
                className="rounded-xl border px-3 py-2 text-sm"
                placeholder="Montant €"
                value={elementForm.montant}
                onChange={(e) => setElementForm({ ...elementForm, montant: e.target.value })}
              />
              <input
                className="rounded-xl border px-3 py-2 text-sm sm:col-span-2 lg:col-span-3"
                placeholder="Libellé"
                value={elementForm.libelle}
                onChange={(e) => setElementForm({ ...elementForm, libelle: e.target.value })}
              />
              <button
                type="button"
                disabled={busy || selected?.statut === "figee"}
                onClick={() =>
                  void post("createElement", {
                    ...elementForm,
                    periodeId,
                  }).then((ok) => {
                    if (ok) setElementForm((f) => ({ ...f, libelle: "", quantite: "", montant: "" }));
                  })
                }
                className="rounded-xl bg-slate-900 px-3 py-2 text-sm font-bold text-white disabled:opacity-50"
              >
                Ajouter
              </button>
            </div>
            <ul className="divide-y text-sm">
              {elements.map((el) => (
                <li key={el.id} className="flex flex-wrap justify-between gap-2 py-2">
                  <span>
                    <span className="mr-2 rounded bg-violet-100 px-1.5 py-0.5 text-[10px] font-bold uppercase text-violet-900">
                      {el.nature}
                    </span>
                    <strong>{el.libelle}</strong>{" "}
                    <span className="text-xs text-slate-500">
                      {el.personnelDisplay ||
                        `${el.personnelPrenom || ""} ${el.personnelNom || ""}`.trim()}
                    </span>
                  </span>
                  <span className="font-mono text-slate-800">
                    {el.quantite != null ? `${el.quantite} · ` : ""}
                    {el.montant != null ? `${el.montant} €` : "—"}
                  </span>
                </li>
              ))}
              {!elements.length && (
                <li className="py-3 text-slate-500">Aucun élément sur cette période.</li>
              )}
            </ul>
          </>
        )}
      </section>
    </ModulePageShell>
  );
}
