"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { formatEncoursMontant, formatFoyerPayeurDetail } from "@/app/lib/foyer-display";

type FoyerFinance = {
  foyer: {
    id: string;
    label: string;
    adresse: string | null;
    codePostal: string | null;
    ville: string | null;
    payeurEstFoyer?: boolean;
  };
  encoursParAnnee?: Array<{
    anneeLabel: string | null;
    montantRestant: number;
    factureCount: number;
  }>;
  facturation: {
    id: string;
    foyerId: string;
    codeAuxiliaire: string | null;
    categorieQuotient: string | null;
    quotientFamilial: string | null;
    iban: string | null;
    bic: string | null;
    rum: string | null;
    mandatDate: string | null;
    acceptePrelevement: boolean;
  } | null;
  factures: Array<{
    id: string;
    numero: string;
    statut: string;
    totalTtc: string;
    dateEmission: string | null;
    dateEcheance?: string | null;
    pdfKey: string | null;
  }>;
};

type Props = {
  eleveId: string;
  canEdit: boolean;
};

export default function EleveFinancesPanel({ eleveId, canEdit }: Props) {
  const [finances, setFinances] = useState<FoyerFinance[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [editFoyerId, setEditFoyerId] = useState<string | null>(null);
  const [form, setForm] = useState({
    codeAuxiliaire: "",
    categorieQuotient: "",
    quotientFamilial: "",
    iban: "",
    bic: "",
    rum: "",
    mandatDate: "",
    acceptePrelevement: false,
  });

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch(`/api/eleves/${eleveId}/finances`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setFinances(data.finances || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, [eleveId]);

  useEffect(() => {
    void load();
  }, [load]);

  const post = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch(`/api/eleves/${eleveId}/finances`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Échec");
      await load();
      return data;
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      return null;
    } finally {
      setBusy(false);
    }
  };

  const startEdit = (block: FoyerFinance) => {
    setEditFoyerId(block.foyer.id);
    setForm({
      codeAuxiliaire: block.facturation?.codeAuxiliaire || "",
      categorieQuotient: block.facturation?.categorieQuotient || "",
      quotientFamilial: block.facturation?.quotientFamilial || "",
      iban: "",
      bic: block.facturation?.bic || "",
      rum: block.facturation?.rum || "",
      mandatDate: block.facturation?.mandatDate || "",
      acceptePrelevement: block.facturation?.acceptePrelevement || false,
    });
  };

  if (!finances.length) {
    return (
      <p className="rounded-2xl border border-slate-200 bg-white p-5 text-sm text-slate-600">
        Aucun foyer lié — renseignez l’onglet Famille d’abord.
      </p>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-xs text-slate-500">
          Facturation foyer — même moteur que le hub Compta.
        </p>
        <Link href="/facturation" className="text-xs font-bold text-indigo-600 hover:underline">
          Hub facturation (SEPA · export CSV)
        </Link>
      </div>
      {message && <p className="text-sm text-emerald-700 font-semibold">{message}</p>}
      {error && <p className="text-sm text-red-600">{error}</p>}

      {finances.map((block) => (
        <section key={block.foyer.id} className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <div className="flex flex-wrap justify-between gap-2 items-start">
            <div>
              <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Foyer</p>
              <h3 className="font-black text-slate-900">{block.foyer.label}</h3>
              <p className="text-xs text-slate-600 mt-1">
                {formatFoyerPayeurDetail({
                  label: block.foyer.label,
                  payeurEstFoyer: block.foyer.payeurEstFoyer !== false,
                  responsables: [],
                })}
              </p>
              <p className="text-xs text-slate-500 mt-1">
                {[block.foyer.adresse, block.foyer.codePostal, block.foyer.ville]
                  .filter(Boolean)
                  .join(" ")}
              </p>
            </div>
            {canEdit && (
              <div className="flex gap-2">
                <button
                  type="button"
                  className="text-xs font-bold text-indigo-600"
                  onClick={() => startEdit(block)}
                >
                  SEPA / compta
                </button>
                <button
                  type="button"
                  disabled={busy}
                  className="rounded-xl bg-indigo-600 text-white px-3 py-1.5 text-xs font-bold disabled:opacity-50"
                  onClick={async () => {
                    const data = await post({
                      action: "createFacture",
                      foyerId: block.foyer.id,
                      autoTarifs: true,
                    });
                    if (data) setMessage(`Facture brouillon créée (${data.facture?.facture?.numero || ""}).`);
                  }}
                >
                  + Facture auto
                </button>
              </div>
            )}
          </div>

          {(block.encoursParAnnee ?? []).some((e) => e.montantRestant > 0) ? (
            <div className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-xs">
              <p className="font-bold text-amber-950">Encours comptable (conservé d’une année à l’autre)</p>
              <ul className="mt-1 space-y-0.5 text-amber-900">
                {block.encoursParAnnee!
                  .filter((e) => e.montantRestant > 0)
                  .map((e) => (
                    <li key={e.anneeLabel ?? "na"}>
                      {e.anneeLabel ?? "Hors année"} : {formatEncoursMontant(e.montantRestant)}
                    </li>
                  ))}
              </ul>
            </div>
          ) : null}

          {editFoyerId === block.foyer.id && canEdit && (
            <div className="grid gap-2 sm:grid-cols-2 text-sm border-t border-slate-100 pt-4">
              <input
                className="border rounded-xl px-3 py-2"
                placeholder="Code auxiliaire 411…"
                value={form.codeAuxiliaire}
                onChange={(e) => setForm({ ...form, codeAuxiliaire: e.target.value })}
              />
              <input
                className="border rounded-xl px-3 py-2"
                placeholder="Quotient familial"
                value={form.quotientFamilial}
                onChange={(e) => setForm({ ...form, quotientFamilial: e.target.value })}
              />
              <input
                className="border rounded-xl px-3 py-2 sm:col-span-2"
                placeholder="IBAN (non stocké en clair dans l’UI après enregistrement)"
                value={form.iban}
                onChange={(e) => setForm({ ...form, iban: e.target.value })}
              />
              <input
                className="border rounded-xl px-3 py-2"
                placeholder="BIC"
                value={form.bic}
                onChange={(e) => setForm({ ...form, bic: e.target.value })}
              />
              <input
                className="border rounded-xl px-3 py-2"
                placeholder="RUM mandat SEPA"
                value={form.rum}
                onChange={(e) => setForm({ ...form, rum: e.target.value })}
              />
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={form.acceptePrelevement}
                  onChange={(e) => setForm({ ...form, acceptePrelevement: e.target.checked })}
                />
                Prélèvement SEPA accepté
              </label>
              <button
                type="button"
                disabled={busy}
                className="rounded-xl bg-slate-900 text-white px-3 py-2 font-bold disabled:opacity-50"
                onClick={async () => {
                  const data = await post({
                    action: "upsertFoyerFacturation",
                    foyerId: block.foyer.id,
                    ...form,
                  });
                  if (data) {
                    setMessage("Coordonnées facturation enregistrées.");
                    setEditFoyerId(null);
                  }
                }}
              >
                Enregistrer
              </button>
            </div>
          )}

          {block.facturation && (
            <div className="text-xs text-slate-600 grid sm:grid-cols-3 gap-2">
              <span>Aux. {block.facturation.codeAuxiliaire || "—"}</span>
              <span>IBAN {block.facturation.iban || "—"}</span>
              <span>RUM {block.facturation.rum || "—"}</span>
            </div>
          )}

          <ul className="divide-y text-sm">
            {block.factures.map((f) => {
              const today = new Date().toISOString().slice(0, 10);
              const retard =
                (f.statut === "emise" || f.statut === "partiellement_payee") &&
                Boolean(f.dateEcheance) &&
                (f.dateEcheance as string) < today;
              return (
              <li key={f.id} className="py-2 flex flex-wrap justify-between gap-2 items-center">
                <span>
                  <strong>{f.numero}</strong>{" "}
                  <span className="text-slate-500">{f.statut}</span>
                  {retard ? (
                    <span className="ml-2 rounded bg-amber-100 px-1.5 py-0.5 text-[10px] font-bold uppercase text-amber-900">
                      Retard
                    </span>
                  ) : null}
                  <span className="block text-xs text-slate-400">
                    émise {f.dateEmission || "—"}
                    {f.dateEcheance ? ` · éch. ${f.dateEcheance}` : ""}
                  </span>
                </span>
                <span className="flex items-center gap-2">
                  <span className="font-semibold">{f.totalTtc} €</span>
                  {canEdit && f.statut === "brouillon" && (
                    <button
                      type="button"
                      className="text-xs font-bold text-indigo-600"
                      disabled={busy}
                      onClick={async () => {
                        await post({ action: "emitFacture", factureId: f.id });
                        setMessage(`Facture ${f.numero} émise.`);
                      }}
                    >
                      Émettre
                    </button>
                  )}
                  {canEdit && (f.statut === "emise" || f.statut === "partiellement_payee") && (
                    <>
                      <button
                        type="button"
                        className="text-xs font-bold text-emerald-700"
                        disabled={busy}
                        onClick={async () => {
                          await post({ action: "solderFacture", factureId: f.id });
                          setMessage(`Facture ${f.numero} soldée.`);
                        }}
                      >
                        Solder
                      </button>
                      <button
                        type="button"
                        className="text-xs font-bold text-amber-800"
                        disabled={busy}
                        onClick={async () => {
                          await post({ action: "noterRelance", factureId: f.id });
                          setMessage(`Relance notée pour ${f.numero}.`);
                        }}
                      >
                        Relance
                      </button>
                    </>
                  )}
                  {canEdit && (
                    <button
                      type="button"
                      className="text-xs font-bold text-indigo-600"
                      disabled={busy}
                      onClick={async () => {
                        await post({ action: "generatePdf", factureId: f.id });
                        setMessage(`PDF généré pour ${f.numero}.`);
                      }}
                    >
                      PDF
                    </button>
                  )}
                  {(f.pdfKey || f.statut !== "brouillon") && (
                    <a
                      href={`/api/facturation/${f.id}/pdf`}
                      target="_blank"
                      rel="noreferrer"
                      className="text-xs font-bold text-slate-700 underline"
                    >
                      Ouvrir
                    </a>
                  )}
                </span>
              </li>
              );
            })}
            {!block.factures.length && (
              <li className="py-2 text-slate-500">Aucune facture — « + Facture auto » depuis les tarifs actifs.</li>
            )}
          </ul>
        </section>
      ))}
    </div>
  );
}
