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
  nature?: string;
};

type Echeance = {
  id: string;
  ordre: number;
  dateEcheance: string;
  montant: string;
};

type Impaye = {
  factureId: string;
  numero: string;
  statut: string;
  foyerLabel: string;
  totalTtc: string;
  paye: string;
  reste: string;
  dateEcheance: string | null;
  enRetard: boolean;
  echeances: Echeance[];
};

type EncaissementRow = {
  id: string;
  montant: string;
  mode: string;
  dateEncaissement: string;
  reference: string | null;
};

export default function FacturationClient() {
  const [tarifs, setTarifs] = useState<Tarif[]>([]);
  const [factures, setFactures] = useState<Facture[]>([]);
  const [impayes, setImpayes] = useState<Impaye[]>([]);
  const [echeancesByFactureId, setEcheancesByFactureId] = useState<Record<string, Echeance[]>>(
    {},
  );
  const [encaissementsByFactureId, setEncaissementsByFactureId] = useState<
    Record<string, EncaissementRow[]>
  >({});
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
      setImpayes(data.impayes || []);
      setEcheancesByFactureId(data.echeancesByFactureId || {});
      setEncaissementsByFactureId(data.encaissementsByFactureId || {});
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

  const factureAction = async (
    action: string,
    factureId: string,
    extra?: Record<string, unknown>,
  ) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/facturation", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action, factureId, ...extra }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      const labels: Record<string, string> = {
        emitFacture: "Facture émise (PDF prêt pour la famille).",
        generatePdf: "PDF généré.",
        solderFacture: "Facture soldée (encaissement enregistré).",
        noterRelance: "Relance notée.",
        annulerFacture: "Facture annulée.",
        createAvoir: "Avoir créé et émis.",
        setEcheancier: "Échéancier enregistré.",
        enregistrerEncaissement: "Encaissement partiel enregistré.",
      };
      setMessage(labels[action] || "OK.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const enRetardCount = impayes.filter((i) => i.enRetard).length;

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Compta & RH · Phase 1b"
        title="Facturation familles"
        description="Catalogue tarifs, factures foyers, échéances, impayés, quittances, SEPA."
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
            {enRetardCount} impayé(s) en retard
          </span>
        ) : null}
      </div>

      <section className="rounded-2xl border border-amber-200 bg-amber-50/40 p-5 space-y-3 mb-6">
        <h2 className="font-black text-slate-900">
          Board impayés ({impayes.length})
        </h2>
        <p className="text-xs text-slate-600">
          Factures émises ou partielles avec reste à payer. La quittance est l’encaissement.
        </p>
        {!impayes.length ? (
          <p className="text-sm text-slate-500">Aucun impayé — tout est soldé.</p>
        ) : (
          <ul className="text-sm divide-y divide-amber-100/80">
            {impayes.map((i) => (
              <li key={i.factureId} className="py-3 space-y-2">
                <div className="flex flex-wrap justify-between gap-2 items-center">
                  <span>
                    <strong>{i.numero}</strong>{" "}
                    <span className="text-slate-500 text-xs">{i.foyerLabel}</span>{" "}
                    <span className="text-slate-500 text-xs">{i.statut}</span>
                    {i.enRetard ? (
                      <span className="ml-2 rounded bg-amber-200 px-1.5 py-0.5 text-[10px] font-bold uppercase text-amber-950">
                        Retard
                      </span>
                    ) : null}
                    {i.dateEcheance ? (
                      <span className="ml-2 text-xs text-slate-500">proch. éch. {i.dateEcheance}</span>
                    ) : null}
                  </span>
                  <span className="font-bold text-amber-950">
                    reste {i.reste} €{" "}
                    <span className="font-normal text-slate-500 text-xs">
                      / {i.totalTtc} € (payé {i.paye} €)
                    </span>
                  </span>
                </div>
                {i.echeances.length > 0 ? (
                  <div className="flex flex-wrap gap-2 text-[11px] text-slate-700">
                    {i.echeances.map((e) => (
                      <span
                        key={e.id}
                        className="rounded-lg border border-amber-200 bg-white px-2 py-1"
                      >
                        #{e.ordre} · {e.dateEcheance} · {e.montant} €
                      </span>
                    ))}
                  </div>
                ) : null}
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="text-xs font-bold text-indigo-700"
                    disabled={busy}
                    onClick={() =>
                      void factureAction("setEcheancier", i.factureId, { nbMensualites: 3 })
                    }
                  >
                    Échéancier 3×
                  </button>
                  <button
                    type="button"
                    className="text-xs font-bold text-emerald-700"
                    disabled={busy}
                    onClick={() => void factureAction("solderFacture", i.factureId)}
                  >
                    Solder
                  </button>
                  <button
                    type="button"
                    className="text-xs font-bold text-slate-700"
                    disabled={busy}
                    onClick={() => {
                      const first = i.echeances[0];
                      const montant = first?.montant || i.reste;
                      void factureAction("enregistrerEncaissement", i.factureId, {
                        montant,
                        mode: "virement",
                        reference: first
                          ? `Échéance #${first.ordre} ${i.numero}`
                          : `Partiel ${i.numero}`,
                      });
                    }}
                  >
                    Encaisser 1ʳᵉ éch.
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>

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
          Émission depuis le dossier foyer. Échéancier multi et quittances sur les pièces ci-dessous.
        </p>
        <ul className="text-sm divide-y">
          {factures.map((f) => {
            const retard =
              (f.statut === "emise" || f.statut === "partiellement_payee") &&
              Boolean(f.dateEcheance) &&
              (f.dateEcheance as string) < today;
            const echs = echeancesByFactureId[f.id] || [];
            const encs = encaissementsByFactureId[f.id] || [];
            return (
              <li key={f.id} className="py-3 space-y-2">
                <div className="flex justify-between gap-3 items-center flex-wrap">
                  <span>
                    <strong>{f.numero}</strong>{" "}
                    <span className="text-slate-500 text-xs">{f.statut}</span>
                    {f.nature === "avoir" ? (
                      <span className="ml-2 rounded bg-violet-100 px-1.5 py-0.5 text-[10px] font-bold uppercase text-violet-900">
                        Avoir
                      </span>
                    ) : null}
                    {f.dateEcheance ? (
                      <span className="ml-2 text-xs text-slate-500">éch. {f.dateEcheance}</span>
                    ) : null}
                    {retard ? (
                      <span className="ml-2 rounded bg-amber-100 px-1.5 py-0.5 text-[10px] font-bold uppercase text-amber-900">
                        Retard
                      </span>
                    ) : null}
                    {echs.length > 1 ? (
                      <span className="ml-2 text-xs text-indigo-700 font-semibold">
                        {echs.length} échéances
                      </span>
                    ) : null}
                  </span>
                  <span className="flex items-center gap-2 flex-wrap">
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
                    {(f.statut === "emise" || f.statut === "partiellement_payee") &&
                      f.nature !== "avoir" && (
                        <>
                          <button
                            type="button"
                            className="text-xs font-bold text-indigo-700"
                            disabled={busy}
                            onClick={() =>
                              void factureAction("setEcheancier", f.id, { nbMensualites: 3 })
                            }
                          >
                            Échéancier 3×
                          </button>
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
                    {(f.statut === "emise" ||
                      f.statut === "partiellement_payee" ||
                      f.statut === "soldee") &&
                      f.nature !== "avoir" && (
                        <button
                          type="button"
                          className="text-xs font-bold text-violet-700"
                          disabled={busy}
                          onClick={() => void factureAction("createAvoir", f.id)}
                        >
                          Avoir
                        </button>
                      )}
                    {f.statut === "emise" && f.nature !== "avoir" && (
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
                </div>
                {echs.length > 0 ? (
                  <div className="flex flex-wrap gap-2 pl-1 text-[11px] text-slate-600">
                    {echs.map((e) => (
                      <span key={e.id} className="rounded bg-slate-50 border border-slate-200 px-2 py-0.5">
                        éch. #{e.ordre} {e.dateEcheance} — {e.montant} €
                      </span>
                    ))}
                  </div>
                ) : null}
                {encs.length > 0 ? (
                  <div className="flex flex-wrap gap-2 pl-1 text-[11px]">
                    {encs.map((e) => (
                      <a
                        key={e.id}
                        href={`/api/facturation/encaissement/${e.id}/quittance`}
                        target="_blank"
                        rel="noreferrer"
                        className="rounded bg-emerald-50 border border-emerald-200 px-2 py-0.5 font-bold text-emerald-900 hover:underline"
                      >
                        Quittance {e.montant} € ({e.dateEncaissement})
                      </a>
                    ))}
                  </div>
                ) : null}
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
