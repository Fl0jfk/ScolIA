"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type Tarif = {
  id: string;
  code: string;
  libelle: string;
  prixUnitaire: string;
  periodicite: string;
  portee: string;
  actif: boolean;
  compteProduit?: string | null;
};

type Facture = {
  id: string;
  numero: string;
  statut: string;
  totalTtc: string;
  foyerId: string;
  dateEmission: string | null;
  dateEcheance: string | null;
};

export default function FacturationClient() {
  const [tarifs, setTarifs] = useState<Tarif[]>([]);
  const [factures, setFactures] = useState<Facture[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [form, setForm] = useState({
    code: "",
    libelle: "",
    prixUnitaire: "0",
    periodicite: "mensuel",
    portee: "regime",
    porteeValeur: "",
    compteProduit: "",
  });
  const [sepaBusy, setSepaBusy] = useState(false);

  const today = new Date().toISOString().slice(0, 10);

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/facturation", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setTarifs(data.tarifs || []);
      setFactures(data.factures || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const saveTarif = async () => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/facturation", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "upsertTarif", ...form }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      setMessage(`Tarif ${data.tarif?.code} enregistré.`);
      setForm({
        code: "",
        libelle: "",
        prixUnitaire: "0",
        periodicite: "mensuel",
        portee: "regime",
        porteeValeur: "",
        compteProduit: "",
      });
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const exportSepa = async () => {
    setSepaBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/facturation/sepa", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error(data?.error || "Export SEPA impossible");
      }
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `sepa-${new Date().toISOString().slice(0, 10)}.xml`;
      a.click();
      URL.revokeObjectURL(url);
      setMessage("Fichier pain.008 téléchargé.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSepaBusy(false);
    }
  };

  const exportComptable = (statut?: string) => {
    const q = statut ? `?statut=${encodeURIComponent(statut)}` : "";
    window.location.href = `/api/facturation/export${q}`;
    setMessage(
      statut === "emise"
        ? "Export comptable (factures émises) lancé."
        : "Export comptable (toutes factures) lancé.",
    );
  };

  const factureAction = async (action: string, factureId: string) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/facturation", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action, factureId }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      const labels: Record<string, string> = {
        emitFacture: "Facture émise.",
        generatePdf: "PDF généré.",
        solderFacture: "Facture soldée (encaissement enregistré).",
        noterRelance: "Relance notée.",
        annulerFacture: "Facture annulée.",
      };
      setMessage(labels[action] || "OK.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const enRetardCount = factures.filter(
    (f) =>
      (f.statut === "emise" || f.statut === "partiellement_payee") &&
      f.dateEcheance &&
      f.dateEcheance < today,
  ).length;

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Compta & RH · Phase 1b"
        title="Facturation familles"
        description="Catalogue tarifs, factures foyers, PDF, SEPA et export comptable CSV."
        actions={
          <Link href="/compta-rh" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Comptabilité & RH
          </Link>
        }
      />

      {message && <p className="mb-3 text-sm text-emerald-700 font-semibold">{message}</p>}
      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}

      <div className="flex flex-wrap gap-3 mb-6">
        <button
          type="button"
          disabled={sepaBusy}
          onClick={() => void exportSepa()}
          className="rounded-xl border border-indigo-200 text-indigo-700 px-4 py-2 text-sm font-bold disabled:opacity-50"
        >
          {sepaBusy ? "Export…" : "Export SEPA (factures émises)"}
        </button>
        <button
          type="button"
          onClick={() => exportComptable("emise")}
          className="rounded-xl border border-emerald-200 text-emerald-800 px-4 py-2 text-sm font-bold"
        >
          Export comptable CSV (émises)
        </button>
        <button
          type="button"
          onClick={() => exportComptable()}
          className="rounded-xl border border-slate-200 text-slate-700 px-4 py-2 text-sm font-bold"
        >
          Export CSV (toutes)
        </button>
        {enRetardCount > 0 ? (
          <span className="rounded-xl bg-amber-50 border border-amber-200 px-3 py-2 text-xs font-bold text-amber-900">
            {enRetardCount} facture(s) en retard
          </span>
        ) : null}
      </div>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4 mb-6">
        <h2 className="font-black text-slate-900">Catalogue tarifs</h2>
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-6">
          <input
            className="border rounded-xl px-3 py-2 text-sm"
            placeholder="Code"
            value={form.code}
            onChange={(e) => setForm({ ...form, code: e.target.value })}
          />
          <input
            className="border rounded-xl px-3 py-2 text-sm sm:col-span-2"
            placeholder="Libellé"
            value={form.libelle}
            onChange={(e) => setForm({ ...form, libelle: e.target.value })}
          />
          <input
            className="border rounded-xl px-3 py-2 text-sm"
            placeholder="Prix"
            value={form.prixUnitaire}
            onChange={(e) => setForm({ ...form, prixUnitaire: e.target.value })}
          />
          <input
            className="border rounded-xl px-3 py-2 text-sm"
            placeholder="Portée (regime/classe)"
            value={form.porteeValeur}
            onChange={(e) => setForm({ ...form, porteeValeur: e.target.value })}
          />
          <input
            className="border rounded-xl px-3 py-2 text-sm"
            placeholder="Compte produit 706…"
            value={form.compteProduit}
            onChange={(e) => setForm({ ...form, compteProduit: e.target.value })}
          />
          <button
            type="button"
            disabled={busy}
            onClick={() => void saveTarif()}
            className="rounded-xl bg-indigo-600 text-white px-3 py-2 text-sm font-bold disabled:opacity-50"
          >
            Enregistrer
          </button>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="text-left text-slate-500">
              <tr>
                <th className="py-2">Code</th>
                <th>Libellé</th>
                <th>Prix</th>
                <th>Périodicité</th>
                <th>Portée</th>
                <th>Compte</th>
              </tr>
            </thead>
            <tbody>
              {tarifs.map((t) => (
                <tr key={t.id} className="border-t border-slate-100">
                  <td className="py-2 font-mono text-xs">{t.code}</td>
                  <td>{t.libelle}</td>
                  <td>{t.prixUnitaire} €</td>
                  <td>{t.periodicite}</td>
                  <td>{t.portee}</td>
                  <td className="font-mono text-xs text-slate-600">{t.compteProduit || "—"}</td>
                </tr>
              ))}
              {!tarifs.length && (
                <tr>
                  <td colSpan={6} className="py-4 text-slate-500">
                    Aucun tarif — créez scolarité, demi-pension, internat…
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
        <h2 className="font-black text-slate-900">Factures ({factures.length})</h2>
        <p className="text-xs text-slate-500">
          Émission depuis le dossier foyer (API prête). Liste des pièces déjà créées :
        </p>
        <ul className="text-sm divide-y">
          {factures.map((f) => {
            const retard =
              (f.statut === "emise" || f.statut === "partiellement_payee") &&
              Boolean(f.dateEcheance) &&
              (f.dateEcheance as string) < today;
            return (
              <li key={f.id} className="py-2 flex justify-between gap-3 items-center flex-wrap">
                <span>
                  <strong>{f.numero}</strong>{" "}
                  <span className="text-slate-500 text-xs">{f.statut}</span>
                  {f.dateEcheance ? (
                    <span className="ml-2 text-xs text-slate-500">éch. {f.dateEcheance}</span>
                  ) : null}
                  {retard ? (
                    <span className="ml-2 rounded bg-amber-100 px-1.5 py-0.5 text-[10px] font-bold uppercase text-amber-900">
                      Retard
                    </span>
                  ) : null}
                </span>
                <span className="flex items-center gap-2">
                  <span>{f.totalTtc} €</span>
                  {f.statut === "brouillon" && (
                    <button
                      type="button"
                      className="text-xs font-bold text-indigo-600"
                      onClick={() => void factureAction("emitFacture", f.id)}
                    >
                      Émettre
                    </button>
                  )}
                  {(f.statut === "emise" || f.statut === "partiellement_payee") && (
                    <>
                      <button
                        type="button"
                        className="text-xs font-bold text-emerald-700"
                        disabled={busy}
                        onClick={() => void factureAction("solderFacture", f.id)}
                      >
                        Solder
                      </button>
                      <button
                        type="button"
                        className="text-xs font-bold text-amber-800"
                        disabled={busy}
                        onClick={() => void factureAction("noterRelance", f.id)}
                      >
                        Relance
                      </button>
                    </>
                  )}
                  {f.statut === "emise" && (
                    <button
                      type="button"
                      className="text-xs font-bold text-slate-500"
                      disabled={busy}
                      onClick={() => void factureAction("annulerFacture", f.id)}
                    >
                      Annuler
                    </button>
                  )}
                  <button
                    type="button"
                    className="text-xs font-bold text-indigo-600"
                    onClick={() => void factureAction("generatePdf", f.id)}
                  >
                    PDF
                  </button>
                  <a
                    href={`/api/facturation/${f.id}/pdf`}
                    target="_blank"
                    rel="noreferrer"
                    className="text-xs font-bold underline text-slate-700"
                  >
                    Ouvrir
                  </a>
                </span>
              </li>
            );
          })}
          {!factures.length && (
            <li className="py-3 text-slate-500">Aucune facture pour l’instant.</li>
          )}
        </ul>
      </section>
    </ModulePageShell>
  );
}
