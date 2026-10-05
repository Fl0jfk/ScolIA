"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type Compte = {
  id: string;
  libelle: string;
  nature: string;
  actif: boolean;
  solde: string;
};

type Depense = {
  id: string;
  dateDepense: string;
  libelle: string;
  fournisseur: string | null;
  portee: string;
  montant: string;
  statut: string;
};

type Mouvement = {
  id: string;
  compteId: string;
  dateMouvement: string;
  sens: string;
  montant: string;
  libelle: string;
  compteLibelle: string;
  compteNature: string;
  depenseId: string | null;
};

type Synthese = {
  from: string;
  to: string;
  recettesLivre: string;
  depensesPayees: string;
  soldePeriode: string;
  encaissementsFamille: string;
  nbMouvements: number;
  nbDepensesPayees: number;
  soldesComptes: Array<{ id: string; libelle: string; nature: string; solde: string }>;
};

export default function ComptaEtablissementClient() {
  const [comptes, setComptes] = useState<Compte[]>([]);
  const [depenses, setDepenses] = useState<Depense[]>([]);
  const [mouvements, setMouvements] = useState<Mouvement[]>([]);
  const [synthese, setSynthese] = useState<Synthese | null>(null);
  const [encaissementsOrphelins, setEncaissementsOrphelins] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const today = new Date().toISOString().slice(0, 10);
  const [periode, setPeriode] = useState({
    from: `${today.slice(0, 8)}01`,
    to: today,
  });

  const [depenseForm, setDepenseForm] = useState({
    libelle: "",
    montant: "",
    portee: "cantine",
    fournisseur: "",
    dateDepense: today,
  });
  const [mouvementForm, setMouvementForm] = useState({
    compteId: "",
    sens: "entree",
    montant: "",
    libelle: "",
    dateMouvement: today,
  });

  const load = useCallback(async () => {
    setError(null);
    try {
      const qs = new URLSearchParams({ from: periode.from, to: periode.to });
      const res = await fetch(`/api/compta?${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setComptes(data.comptes || []);
      setDepenses(data.depenses || []);
      setMouvements(data.mouvements || []);
      setSynthese(data.synthese || null);
      setEncaissementsOrphelins(Number(data.encaissementsOrphelins || 0));
      setMouvementForm((f) =>
        f.compteId || !(data.comptes || []).length
          ? f
          : { ...f, compteId: data.comptes[0].id },
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, [periode.from, periode.to]);

  useEffect(() => {
    void load();
  }, [load]);

  const post = async (action: string, body: Record<string, unknown> = {}) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/compta", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action, ...body }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      const labels: Record<string, string> = {
        ensureComptesDefaut: "Comptes Caisse et Banque créés.",
        createDepense: "Dépense enregistrée.",
        payerDepense: "Dépense payée — sortie de trésorerie créée.",
        createMouvement: "Mouvement enregistré.",
        upsertCompte: "Compte enregistré.",
        importerEncaissements: `${data.imported ?? 0} encaissement(s) importé(s) dans le livre.`,
      };
      setMessage(labels[action] || "OK.");
      await load();
      return data;
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      return null;
    } finally {
      setBusy(false);
    }
  };

  const defaultCompteId = comptes.find((c) => c.nature === "caisse")?.id || comptes[0]?.id || "";
  const caisseId = comptes.find((c) => c.nature === "caisse")?.id || defaultCompteId;
  const banqueId = comptes.find((c) => c.nature === "banque")?.id || defaultCompteId;

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Compta & RH · Partie simple"
        title="Comptabilité d’établissement"
        description="Caisse, banque, dépenses, recettes familles et export cabinet — pas de plan comptable ni de bilan."
        actions={
          <Link href="/compta-rh" className="text-sm font-bold text-emerald-700 hover:underline">
            ← Comptabilité & RH
          </Link>
        }
      />

      {message && <p className="mb-3 text-sm font-semibold text-emerald-700">{message}</p>}
      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}

      <section className="mb-6 space-y-3 rounded-2xl border border-sky-200 bg-sky-50/50 p-5">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <h2 className="font-black text-slate-900">Synthèse période</h2>
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <label className="text-xs text-slate-600">
              Du{" "}
              <input
                type="date"
                className="rounded-lg border px-2 py-1"
                value={periode.from}
                onChange={(e) => setPeriode({ ...periode, from: e.target.value })}
              />
            </label>
            <label className="text-xs text-slate-600">
              au{" "}
              <input
                type="date"
                className="rounded-lg border px-2 py-1"
                value={periode.to}
                onChange={(e) => setPeriode({ ...periode, to: e.target.value })}
              />
            </label>
            <a
              href={`/api/compta/export?from=${encodeURIComponent(periode.from)}&to=${encodeURIComponent(periode.to)}`}
              className="rounded-xl border border-sky-300 bg-white px-3 py-1.5 text-xs font-bold text-sky-900"
            >
              Export cabinet CSV
            </a>
          </div>
        </div>
        {synthese ? (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4 text-sm">
            <div className="rounded-xl bg-white border border-sky-100 p-3">
              <p className="text-xs text-slate-500">Recettes livre</p>
              <p className="font-mono font-bold text-emerald-800">{synthese.recettesLivre} €</p>
            </div>
            <div className="rounded-xl bg-white border border-sky-100 p-3">
              <p className="text-xs text-slate-500">Dépenses payées</p>
              <p className="font-mono font-bold text-rose-800">{synthese.depensesPayees} €</p>
            </div>
            <div className="rounded-xl bg-white border border-sky-100 p-3">
              <p className="text-xs text-slate-500">Solde période (livre)</p>
              <p className="font-mono font-bold text-slate-900">{synthese.soldePeriode} €</p>
            </div>
            <div className="rounded-xl bg-white border border-sky-100 p-3">
              <p className="text-xs text-slate-500">Encaissements familles</p>
              <p className="font-mono font-bold text-indigo-800">{synthese.encaissementsFamille} €</p>
            </div>
          </div>
        ) : (
          <p className="text-sm text-slate-500">Chargement synthèse…</p>
        )}
        <div className="flex flex-wrap gap-2 items-center">
          <button
            type="button"
            disabled={busy || !comptes.length}
            onClick={() => void post("importerEncaissements", { compteId: banqueId })}
            className="rounded-xl bg-indigo-700 px-3 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Importer encaissements → livre
            {encaissementsOrphelins > 0 ? ` (${encaissementsOrphelins})` : ""}
          </button>
          <span className="text-xs text-slate-600">
            Les virements familles alimentent la banque ; déjà liés = ignorés.
          </span>
        </div>
      </section>

      <section className="mb-6 space-y-3 rounded-2xl border border-emerald-200 bg-white p-5">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="font-black text-slate-900">Comptes ({comptes.length})</h2>
          {!comptes.length ? (
            <button
              type="button"
              disabled={busy}
              onClick={() => void post("ensureComptesDefaut")}
              className="rounded-xl bg-emerald-600 px-3 py-2 text-sm font-bold text-white disabled:opacity-50"
            >
              Créer Caisse + Banque
            </button>
          ) : null}
        </div>
        {!comptes.length ? (
          <p className="text-sm text-slate-500">Aucun compte — créez la caisse et la banque pour démarrer.</p>
        ) : (
          <ul className="divide-y text-sm">
            {comptes.map((c) => (
              <li key={c.id} className="flex flex-wrap items-center justify-between gap-2 py-2">
                <span>
                  <strong>{c.libelle}</strong>{" "}
                  <span className="rounded bg-slate-100 px-1.5 py-0.5 text-[10px] font-bold uppercase text-slate-700">
                    {c.nature}
                  </span>
                </span>
                <span className="font-mono font-bold text-emerald-900">{c.solde} €</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="mb-6 space-y-3 rounded-2xl border border-slate-200 bg-white p-5">
        <h2 className="font-black text-slate-900">Nouvelle dépense</h2>
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
          <input
            className="rounded-xl border px-3 py-2 text-sm sm:col-span-2"
            placeholder="Libellé"
            value={depenseForm.libelle}
            onChange={(e) => setDepenseForm({ ...depenseForm, libelle: e.target.value })}
          />
          <input
            className="rounded-xl border px-3 py-2 text-sm"
            placeholder="Montant"
            value={depenseForm.montant}
            onChange={(e) => setDepenseForm({ ...depenseForm, montant: e.target.value })}
          />
          <select
            className="rounded-xl border px-3 py-2 text-sm"
            value={depenseForm.portee}
            onChange={(e) => setDepenseForm({ ...depenseForm, portee: e.target.value })}
          >
            <option value="cantine">Cantine</option>
            <option value="internat">Internat</option>
            <option value="voyage">Voyage</option>
            <option value="scolarite">Scolarité</option>
            <option value="autre">Autre</option>
          </select>
          <input
            type="date"
            className="rounded-xl border px-3 py-2 text-sm"
            value={depenseForm.dateDepense}
            onChange={(e) => setDepenseForm({ ...depenseForm, dateDepense: e.target.value })}
          />
          <input
            className="rounded-xl border px-3 py-2 text-sm sm:col-span-2"
            placeholder="Fournisseur (optionnel)"
            value={depenseForm.fournisseur}
            onChange={(e) => setDepenseForm({ ...depenseForm, fournisseur: e.target.value })}
          />
          <button
            type="button"
            disabled={busy}
            onClick={() =>
              void post("createDepense", { ...depenseForm }).then((ok) => {
                if (ok) {
                  setDepenseForm({
                    libelle: "",
                    montant: "",
                    portee: "cantine",
                    fournisseur: "",
                    dateDepense: new Date().toISOString().slice(0, 10),
                  });
                }
              })
            }
            className="rounded-xl bg-slate-900 px-3 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer dépense
          </button>
        </div>

        <h3 className="pt-2 text-sm font-bold text-slate-800">Dépenses ({depenses.length})</h3>
        <ul className="divide-y text-sm">
          {depenses.map((d) => (
            <li key={d.id} className="flex flex-wrap items-center justify-between gap-2 py-2">
              <span>
                <strong>{d.libelle}</strong>{" "}
                <span className="text-xs text-slate-500">
                  {d.dateDepense} · {d.portee}
                  {d.fournisseur ? ` · ${d.fournisseur}` : ""}
                </span>{" "}
                <span
                  className={`ml-1 rounded px-1.5 py-0.5 text-[10px] font-bold uppercase ${
                    d.statut === "payee"
                      ? "bg-emerald-100 text-emerald-900"
                      : "bg-amber-100 text-amber-900"
                  }`}
                >
                  {d.statut}
                </span>
              </span>
              <span className="flex items-center gap-2">
                <span className="font-mono">{d.montant} €</span>
                {d.statut === "prevue" && comptes.length > 0 ? (
                  <button
                    type="button"
                    disabled={busy}
                    className="text-xs font-bold text-emerald-700"
                    onClick={() =>
                      void post("payerDepense", {
                        depenseId: d.id,
                        compteId: caisseId,
                      })
                    }
                  >
                    Payer (caisse)
                  </button>
                ) : null}
              </span>
            </li>
          ))}
          {!depenses.length && (
            <li className="py-3 text-slate-500">Aucune dépense pour l’instant.</li>
          )}
        </ul>
      </section>

      <section className="mb-6 space-y-3 rounded-2xl border border-slate-200 bg-white p-5">
        <h2 className="font-black text-slate-900">Mouvement manuel</h2>
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
          <select
            className="rounded-xl border px-3 py-2 text-sm"
            value={mouvementForm.compteId || defaultCompteId}
            onChange={(e) => setMouvementForm({ ...mouvementForm, compteId: e.target.value })}
            disabled={!comptes.length}
          >
            {comptes.map((c) => (
              <option key={c.id} value={c.id}>
                {c.libelle} ({c.nature})
              </option>
            ))}
          </select>
          <select
            className="rounded-xl border px-3 py-2 text-sm"
            value={mouvementForm.sens}
            onChange={(e) => setMouvementForm({ ...mouvementForm, sens: e.target.value })}
          >
            <option value="entree">Entrée</option>
            <option value="sortie">Sortie</option>
          </select>
          <input
            className="rounded-xl border px-3 py-2 text-sm"
            placeholder="Montant"
            value={mouvementForm.montant}
            onChange={(e) => setMouvementForm({ ...mouvementForm, montant: e.target.value })}
          />
          <input
            type="date"
            className="rounded-xl border px-3 py-2 text-sm"
            value={mouvementForm.dateMouvement}
            onChange={(e) => setMouvementForm({ ...mouvementForm, dateMouvement: e.target.value })}
          />
          <input
            className="rounded-xl border px-3 py-2 text-sm sm:col-span-2 lg:col-span-3"
            placeholder="Libellé"
            value={mouvementForm.libelle}
            onChange={(e) => setMouvementForm({ ...mouvementForm, libelle: e.target.value })}
          />
          <button
            type="button"
            disabled={busy || !comptes.length}
            onClick={() =>
              void post("createMouvement", {
                ...mouvementForm,
                compteId: mouvementForm.compteId || defaultCompteId,
              }).then((ok) => {
                if (ok) {
                  setMouvementForm((f) => ({
                    ...f,
                    montant: "",
                    libelle: "",
                  }));
                }
              })
            }
            className="rounded-xl bg-emerald-700 px-3 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer
          </button>
        </div>

        <h3 className="pt-2 text-sm font-bold text-slate-800">Livre ({mouvements.length})</h3>
        <ul className="divide-y text-sm">
          {mouvements.map((m) => (
            <li key={m.id} className="flex flex-wrap items-center justify-between gap-2 py-2">
              <span>
                <span
                  className={`mr-2 rounded px-1.5 py-0.5 text-[10px] font-bold uppercase ${
                    m.sens === "entree"
                      ? "bg-emerald-100 text-emerald-900"
                      : "bg-rose-100 text-rose-900"
                  }`}
                >
                  {m.sens}
                </span>
                <strong>{m.libelle}</strong>{" "}
                <span className="text-xs text-slate-500">
                  {m.dateMouvement} · {m.compteLibelle}
                </span>
              </span>
              <span
                className={`font-mono font-bold ${
                  m.sens === "entree" ? "text-emerald-800" : "text-rose-800"
                }`}
              >
                {m.sens === "entree" ? "+" : "−"}
                {m.montant} €
              </span>
            </li>
          ))}
          {!mouvements.length && (
            <li className="py-3 text-slate-500">Aucun mouvement pour l’instant.</li>
          )}
        </ul>
      </section>
    </ModulePageShell>
  );
}
