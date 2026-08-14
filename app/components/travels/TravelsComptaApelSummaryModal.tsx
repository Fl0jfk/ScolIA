"use client";

import { useCallback, useEffect, useState } from "react";
import { TripButton } from "@/app/components/travels/TripDetailUI";
import { formatEuroDisplay } from "@/app/lib/travels-compta-sheet";
import type { TravelsComptaSheet } from "@/app/lib/travels-compta-sheet";
import {
  type ComptaApelEtablissementGroup,
  type ComptaApelSummary,
  type ComptaApelTripCommitment,
} from "@/app/lib/travels-compta-apel-summary";
import { establishmentKindEmoji, inferEstablishmentKind } from "@/app/lib/establishment-visual";

type Props = {
  tripId: string;
  open: boolean;
  onClose: () => void;
  summary: ComptaApelSummary | null;
  loading: boolean;
  onRefresh: () => void;
  currentSheet: TravelsComptaSheet;
};

export default function TravelsComptaApelSummaryModal({
  tripId,
  open,
  onClose,
  summary,
  loading,
  onRefresh,
  currentSheet,
}: Props) {
  const [pdfBusy, setPdfBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) onRefresh();
  }, [open, onRefresh]);

  const downloadPdf = useCallback(async () => {
    setPdfBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/travels/compta-apel-summary/pdf", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, currentSheet }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      const href = String(data.pdf || "");
      if (!href) throw new Error("PDF vide.");
      const link = document.createElement("a");
      link.href = href;
      link.download = String(data.filename || "Recap_APEL.pdf");
      link.click();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Impossible de générer le PDF.");
    } finally {
      setPdfBusy(false);
    }
  }, [tripId, currentSheet]);

  if (!open) return null;

  function ApelTripTable({ trips }: { trips: ComptaApelTripCommitment[] }) {
    return (
      <table className="w-full text-sm">
        <thead>
          <tr className="bg-slate-50 border-b border-slate-100 text-left text-xs text-slate-500">
            <th className="p-3 font-bold">Voyage</th>
            <th className="p-3 font-bold">Date</th>
            <th className="p-3 font-bold text-right">Collectif</th>
            <th className="p-3 font-bold text-right">Individ.</th>
            <th className="p-3 font-bold text-right">Total</th>
          </tr>
        </thead>
        <tbody>
          {trips.map((row) => (
            <tr key={row.tripId} className="border-b border-slate-50">
              <td className="p-3 text-slate-800">{row.title}</td>
              <td className="p-3 text-slate-500 whitespace-nowrap">{row.travelDateLabel}</td>
              <td className="p-3 font-mono text-right whitespace-nowrap">
                {row.apelCollective > 0 ? `${formatEuroDisplay(row.apelCollective)} €` : "—"}
              </td>
              <td className="p-3 font-mono text-right whitespace-nowrap">
                {row.aidesIndividuelles > 0 ? `${formatEuroDisplay(row.aidesIndividuelles)} €` : "—"}
              </td>
              <td className="p-3 font-mono text-right font-semibold whitespace-nowrap">
                {row.totalApel > 0 ? `${formatEuroDisplay(row.totalApel)} €` : "—"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    );
  }

  function etabEmoji(etab: string): string {
    return establishmentKindEmoji(inferEstablishmentKind({ label: etab }));
  }

  return (
    <div className="fixed inset-0 z-[210] flex items-center justify-center bg-black/50 p-4">
      <div className="flex max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl bg-white shadow-2xl">
        <div className="border-b border-emerald-100 bg-emerald-50 px-5 py-4 flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="text-xs font-black uppercase tracking-widest text-emerald-800">APEL</p>
            <h3 className="text-lg font-black text-slate-900">Total général à facturer à l&apos;APEL</h3>
            <p className="mt-1 text-sm text-slate-600">
              Année scolaire {summary?.schoolYear.label ?? "…"} — du 1<sup>er</sup> septembre au 15 juillet.
              <span className="block text-xs text-slate-500 mt-0.5">
                Récapitulatif par niveau (école, collège, lycée, groupe scolaire) pour le versement à l&apos;APEL.
              </span>
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg px-2 py-1 text-slate-400 hover:bg-white hover:text-slate-700"
            aria-label="Fermer"
          >
            ✕
          </button>
        </div>

        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
          {error && (
            <div className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</div>
          )}
          {loading ? (
            <p className="text-center text-slate-500 py-8">Chargement du récapitulatif…</p>
          ) : summary ? (
            <>
              <div className="rounded-xl border border-emerald-200 overflow-hidden">
                <div className="bg-emerald-50 px-4 py-2 text-xs font-black uppercase tracking-widest text-emerald-800">
                  Synthèse par niveau — versement APEL
                </div>
                <table className="w-full text-sm">
                  <thead>
                    <tr className="bg-white border-b border-emerald-100 text-left text-xs text-slate-500">
                      <th className="p-3 font-bold">Niveau</th>
                      <th className="p-3 font-bold text-right">Collectif</th>
                      <th className="p-3 font-bold text-right">Individ.</th>
                      <th className="p-3 font-bold text-right">Total à verser</th>
                    </tr>
                  </thead>
                  <tbody>
                    {summary.byEtablissement.map((group) => {
                      const etab = group.etablissement;
                      const t = group.totals;
                      return (
                        <tr key={etab} className="border-b border-emerald-50">
                          <td className="p-3 font-medium text-slate-800">
                            {etabEmoji(etab)} {etab}
                          </td>
                          <td className="p-3 font-mono text-right whitespace-nowrap">
                            {(t?.apelCollective ?? 0) > 0
                              ? `${formatEuroDisplay(t?.apelCollective ?? 0)} €`
                              : "—"}
                          </td>
                          <td className="p-3 font-mono text-right whitespace-nowrap">
                            {(t?.aidesIndividuelles ?? 0) > 0
                              ? `${formatEuroDisplay(t?.aidesIndividuelles ?? 0)} €`
                              : "—"}
                          </td>
                          <td className="p-3 font-mono text-right font-semibold whitespace-nowrap">
                            {(t?.totalApel ?? 0) > 0
                              ? `${formatEuroDisplay(t?.totalApel ?? 0)} €`
                              : "—"}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                  <tfoot>
                    <tr className="bg-emerald-50 font-bold text-emerald-950">
                      <td className="p-3">Total général</td>
                      <td className="p-3 font-mono text-right whitespace-nowrap">
                        {formatEuroDisplay(summary.totals.apelCollective)} €
                      </td>
                      <td className="p-3 font-mono text-right whitespace-nowrap">
                        {formatEuroDisplay(summary.totals.aidesIndividuelles)} €
                      </td>
                      <td className="p-3 font-mono text-right whitespace-nowrap">
                        {formatEuroDisplay(summary.totals.totalApel)} €
                      </td>
                    </tr>
                  </tfoot>
                </table>
              </div>

              {summary.byEtablissement.length > 0 ? (
                summary.byEtablissement.map((group: ComptaApelEtablissementGroup) => (
                  <div key={group.etablissement} className="rounded-xl border border-slate-200 overflow-hidden">
                    <div className="bg-slate-50 px-4 py-2 flex flex-wrap justify-between items-center gap-2 border-b border-slate-100">
                      <p className="text-xs font-black uppercase tracking-widest text-slate-600">
                        {etabEmoji(group.etablissement)} {group.etablissement}
                      </p>
                      <p className="font-mono font-bold text-slate-800 whitespace-nowrap">
                        {formatEuroDisplay(group.totals.totalApel)} €
                      </p>
                    </div>
                    <ApelTripTable trips={group.trips} />
                  </div>
                ))
              ) : (
                <p className="text-center text-slate-400 italic py-4">
                  Aucun engagement APEL renseigné sur les voyages de cette année scolaire.
                </p>
              )}
            </>
          ) : (
            <p className="text-center text-slate-500 py-8">Récapitulatif indisponible.</p>
          )}
        </div>

        <div className="border-t border-slate-100 px-5 py-3 flex flex-wrap justify-end gap-2 bg-slate-50">
          <TripButton variant="dark" size="sm" onClick={() => void downloadPdf()} disabled={pdfBusy || loading}>
            {pdfBusy ? "PDF…" : "Télécharger le récap PDF"}
          </TripButton>
          <TripButton variant="secondary" size="sm" onClick={onClose}>
            Fermer
          </TripButton>
        </div>
      </div>
    </div>
  );
}
