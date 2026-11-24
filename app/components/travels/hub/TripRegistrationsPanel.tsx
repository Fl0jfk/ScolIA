"use client";

/**
 * Vue « Inscriptions » (circuit v2, lot 2) : une ligne par invitation, avec son statut.
 * Visible par l'organisateur, la direction, l'administratif, les admins et la compta (qui voit qui a signé et le montant).
 * Le suivi complet (relances ciblées, second parent, aides) viendra au lot 3.
 */

import { useCallback, useEffect, useState } from "react";
import { TripAlert, TripButton, TripSection } from "@/app/components/travels/TripDetailUI";
import type { RegistrationRow } from "@/app/lib/travels-v2-registration-store";
import { dateFr } from "@/app/lib/travels-family-page";

type Payload = {
  opened: boolean;
  status: string;
  deadline: string | null;
  rows: RegistrationRow[];
  counts: Record<string, number>;
  engagedStudents: number;
  engagedTotal?: number;
  canWrite: boolean;
  canSeeAmount: boolean;
  smtpOk: boolean;
};

const TONES: Record<string, string> = {
  pas_ouvert: "bg-slate-100 text-slate-600",
  ouvert: "bg-sky-100 text-sky-800",
  en_cours: "bg-indigo-100 text-indigo-800",
  bloque_piece: "bg-rose-100 text-rose-800",
  pret: "bg-amber-100 text-amber-900",
  signe_oui: "bg-emerald-100 text-emerald-800",
  refus: "bg-zinc-200 text-zinc-700",
  sans_email: "bg-orange-100 text-orange-900",
};

function eur(n: number): string {
  return `${n.toFixed(2).replace(".", ",")} €`;
}

export function TripRegistrationsPanel({ tripId }: { tripId: string }) {
  const [data, setData] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [filter, setFilter] = useState<string>("all");

  const load = useCallback(async () => {
    try {
      const res = await fetch(`/api/travels/v2/invitations?tripId=${encodeURIComponent(tripId)}`, { cache: "no-store" });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setData(j as Payload);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement impossible");
    }
  }, [tripId]);

  useEffect(() => {
    void load();
  }, [load]);

  const act = async (body: Record<string, unknown>, okMsg?: string): Promise<{ ok: boolean; url?: string }> => {
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/travels/v2/invitations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, ...body }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Action impossible");
      if (okMsg) setMsg(okMsg);
      await load();
      return { ok: true, url: j.url };
    } catch (e) {
      setError(e instanceof Error ? e.message : "Action impossible");
      return { ok: false };
    } finally {
      setBusy(false);
    }
  };

  const download = async (invitationId: string, opts: { docId?: string; engagement?: boolean }) => {
    const r = await act({ action: "download", invitationId, ...opts });
    if (r.ok && r.url) window.open(r.url, "_blank", "noopener");
  };

  if (error && !data) return <TripAlert tone="warning" title="Inscriptions">{error}</TripAlert>;
  if (!data) return <p className="text-sm text-slate-500">Chargement des inscriptions…</p>;
  if (!data.opened) return null;

  const rows = data.rows.filter((r) => filter === "all" || r.status === filter);
  const open = data.status === "INSCRIPTIONS_OUVERTES";
  const count = (k: string) => data.counts[k] || 0;
  const withoutEmail = data.rows.filter((r) => r.status === "sans_email").length;
  const neverSent = data.rows.filter((r) => r.invitationId && !r.sentAt).length;

  return (
    <TripSection
      title="Inscriptions des familles"
      subtitle={data.deadline ? `Date limite de réponse : ${dateFr(data.deadline)}` : undefined}
      icon="✍️"
      accent="emerald"
    >
      <div className="space-y-4">
        {error ? <TripAlert tone="warning" title="Erreur">{error}</TripAlert> : null}
        {msg ? <TripAlert tone="success" title={msg} /> : null}
        {!data.smtpOk ? (
          <TripAlert tone="warning" title="Envoi d'e-mails non configuré">
            Les liens ne peuvent pas partir tant que l&apos;envoi d&apos;e-mails n&apos;est pas configuré pour l&apos;établissement.
          </TripAlert>
        ) : null}

        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          <div className="rounded-xl border bg-white p-3">
            <p className="text-xs text-slate-500">Élèves inscrits</p>
            <p className="text-2xl font-black text-emerald-700">{data.engagedStudents}</p>
          </div>
          <div className="rounded-xl border bg-white p-3">
            <p className="text-xs text-slate-500">Refus</p>
            <p className="text-2xl font-black text-zinc-700">{count("refus")}</p>
          </div>
          <div className="rounded-xl border bg-white p-3">
            <p className="text-xs text-slate-500">Bloqués par une pièce</p>
            <p className="text-2xl font-black text-rose-700">{count("bloque_piece")}</p>
          </div>
          {data.canSeeAmount ? (
            <div className="rounded-xl border bg-white p-3">
              <p className="text-xs text-slate-500">Engagements signés</p>
              <p className="text-2xl font-black text-indigo-700">{eur(data.engagedTotal ?? 0)}</p>
            </div>
          ) : (
            <div className="rounded-xl border bg-white p-3">
              <p className="text-xs text-slate-500">Pas encore ouverts</p>
              <p className="text-2xl font-black text-slate-700">{count("pas_ouvert")}</p>
            </div>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <select value={filter} onChange={(e) => setFilter(e.target.value)} className="rounded-lg border bg-white p-2 text-sm" aria-label="Filtrer par statut">
            <option value="all">Tous les statuts</option>
            {Object.entries({
              pas_ouvert: "Pas ouvert",
              ouvert: "Ouvert",
              en_cours: "En cours",
              bloque_piece: "Bloqué par une pièce",
              pret: "Prêt à signer",
              signe_oui: "Signé — participe",
              refus: "Refus",
              sans_email: "Sans e-mail",
            }).map(([k, l]) => (
              <option key={k} value={k}>
                {l}
              </option>
            ))}
          </select>
          {data.canWrite && open ? (
            <>
              {neverSent > 0 ? (
                <TripButton size="sm" variant="secondary" disabled={busy} onClick={() => void act({ action: "send_pending" }, "Envoi des liens en cours…")}>
                  Envoyer les liens non partis ({neverSent})
                </TripButton>
              ) : null}
              <TripButton size="sm" variant="secondary" disabled={busy} onClick={() => void act({ action: "create_missing" }, "Invitations mises à jour, envoi en cours…")}>
                Actualiser avec les fiches élèves
              </TripButton>
            </>
          ) : null}
          <TripButton size="sm" variant="secondary" disabled={busy} onClick={() => void load()}>
            Rafraîchir
          </TripButton>
        </div>

        {withoutEmail > 0 ? (
          <TripAlert tone="warning" title={`${withoutEmail} élève(s) sans e-mail de responsable`}>
            Aucun lien ne peut leur être envoyé : complétez la fiche élève puis « Actualiser avec les fiches élèves ».
          </TripAlert>
        ) : null}

        <div className="overflow-x-auto">
          <table className="w-full min-w-[640px] border-collapse text-left text-sm">
            <thead>
              <tr className="border-b text-xs uppercase text-slate-500">
                <th className="py-2 pr-2">Élève</th>
                <th className="py-2 pr-2">Responsable</th>
                <th className="py-2 pr-2">Statut</th>
                <th className="py-2 pr-2">Signature</th>
                {data.canSeeAmount ? <th className="py-2 pr-2 text-right">Montant</th> : null}
                <th className="py-2" />
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={`${r.ine}-${r.invitationId ?? "x"}`} className="border-b align-top">
                  <td className="py-2 pr-2">
                    <span className="font-semibold">{r.studentName}</span>
                    {r.classe ? <span className="ml-1 text-xs text-slate-500">({r.classe})</span> : null}
                  </td>
                  <td className="py-2 pr-2 text-xs text-slate-600">
                    {r.email ?? "—"}
                    {r.lastSendError ? <span className="block text-rose-600">Envoi en échec</span> : null}
                    {r.invitationId && !r.sentAt && !r.lastSendError ? <span className="block text-amber-700">Lien pas encore envoyé</span> : null}
                  </td>
                  <td className="py-2 pr-2">
                    <span className={`rounded-full px-2 py-0.5 text-[11px] font-bold ${TONES[r.status] || ""}`}>{r.statusLabel}</span>
                    {r.status === "bloque_piece" ? (
                      <span className="mt-1 block text-[11px] text-rose-700">
                        Manque : {r.documents.filter((d) => d.kind === "avant_signature" && !d.uploaded && !d.exempt).map((d) => d.label).join(", ")}
                      </span>
                    ) : null}
                    {r.extendedDeadline ? <span className="mt-1 block text-[11px] text-slate-500">Prolongée jusqu&apos;au {dateFr(r.extendedDeadline)}</span> : null}
                  </td>
                  <td className="py-2 pr-2 text-xs text-slate-600">
                    {r.signedAt ? (
                      <>
                        {r.signerName}
                        <span className="block">{new Date(r.signedAt).toLocaleString("fr-FR", { timeZone: "Europe/Paris", dateStyle: "short", timeStyle: "short" })}</span>
                      </>
                    ) : (
                      "—"
                    )}
                  </td>
                  {data.canSeeAmount ? <td className="py-2 pr-2 text-right font-semibold">{typeof r.amount === "number" ? eur(r.amount) : "—"}</td> : null}
                  <td className="py-2">
                    {r.invitationId ? (
                      <div className="flex flex-col items-start gap-1">
                        {r.status === "signe_oui" ? (
                          <button type="button" className="text-xs font-semibold text-indigo-600 underline" onClick={() => void download(r.invitationId!, { engagement: true })}>
                            Engagement (PDF)
                          </button>
                        ) : null}
                        {data.canWrite && open && !r.signedAt ? (
                          <>
                            <button type="button" className="text-xs font-semibold text-indigo-600 underline" disabled={busy} onClick={() => void act({ action: "resend", invitationId: r.invitationId }, "Lien renvoyé.")}>
                              Renvoyer le lien
                            </button>
                            {r.documents.filter((d) => d.kind === "avant_signature" && !d.uploaded).map((d) => (
                              <button
                                key={d.id}
                                type="button"
                                className="text-left text-xs font-semibold text-indigo-600 underline"
                                disabled={busy}
                                onClick={() => void act({ action: "exempt", invitationId: r.invitationId, docId: d.id, value: !d.exempt }, d.exempt ? "Dispense retirée." : "Dispense accordée.")}
                              >
                                {d.exempt ? `Retirer la dispense : ${d.label}` : `Dispenser : ${d.label}`}
                              </button>
                            ))}
                            <button
                              type="button"
                              className="text-xs font-semibold text-indigo-600 underline"
                              disabled={busy}
                              onClick={() => {
                                const date = window.prompt("Nouvelle date limite pour cette famille (AAAA-MM-JJ) :", r.extendedDeadline || data.deadline || "");
                                if (date) void act({ action: "extend", invitationId: r.invitationId, date }, "Date limite prolongée.");
                              }}
                            >
                              Prolonger la date limite
                            </button>
                            {data.canWrite
                              ? r.documents.filter((d) => d.uploaded).map((d) => (
                                  <button key={d.id} type="button" className="text-left text-xs font-semibold text-slate-600 underline" onClick={() => void download(r.invitationId!, { docId: d.id })}>
                                    Voir : {d.label}
                                  </button>
                                ))
                              : null}
                          </>
                        ) : null}
                      </div>
                    ) : null}
                  </td>
                </tr>
              ))}
              {rows.length === 0 ? (
                <tr>
                  <td colSpan={6} className="py-6 text-center text-sm text-slate-500">
                    Aucune invitation pour ce filtre.
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
      </div>
    </TripSection>
  );
}
