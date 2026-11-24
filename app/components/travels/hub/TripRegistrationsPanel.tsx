"use client";

/**
 * Suivi établissement des inscriptions (circuit v2) : statut par famille, filtres, relances ciblées (avec
 * confirmation humaine), export CSV / Excel, coupon papier, désaccords entre responsables (la direction tranche).
 * Visible par l'organisateur, la direction, l'administratif, les admins et la compta (montant).
 */

import { useCallback, useEffect, useState } from "react";
import { TripAlert, TripButton, TripSection } from "@/app/components/travels/TripDetailUI";
import type { RegistrationRow } from "@/app/lib/travels-v2-registration-store";
import { dateFr } from "@/app/lib/travels-family-page";
import { rowMatchesFilter, type FollowupFilter } from "@/app/lib/travels-v2-lot3";
import { MEAL_TYPE_LABELS, MEAL_TYPE_VALUES, type MealType } from "@/app/lib/travels-v2-registration-types";

type Payload = {
  opened: boolean;
  status: string;
  deadline: string | null;
  rows: RegistrationRow[];
  counts: Record<string, number>;
  engagedStudents: number;
  engagedTotal?: number;
  suspended: number;
  withoutEmail: number;
  picnicDays: { key: string; label: string }[];
  isDirection: boolean;
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
  desaccord: "bg-red-100 text-red-800",
  suspens: "bg-red-100 text-red-800",
  retire: "bg-zinc-200 text-zinc-700",
  desiste: "bg-zinc-200 text-zinc-700",
  sans_email: "bg-orange-100 text-orange-900",
};

const FILTER_LABELS: Record<FollowupFilter, string> = {
  pas_ouvert: "Pas ouvert",
  ouvert: "Ouvert",
  en_cours: "En cours (pièces X/Y)",
  bloque_piece: "Bloqué par une pièce",
  pret: "Prêt à signer",
  signe_oui: "Signé — participe",
  refus: "Refus",
  suspens: "En suspens",
  desaccord: "Désaccord",
  retire: "Retiré",
  desiste: "Désisté",
  dispense: "Dispensé d'une pièce",
  papier: "Réponse papier",
  sans_email: "Sans e-mail",
};

/** Statuts pour lesquels une relance ciblée « toutes les familles de ce statut » est proposée. */
const REMIND_FILTERS: FollowupFilter[] = ["pas_ouvert", "ouvert", "en_cours", "bloque_piece", "pret"];

function eur(n: number): string {
  return `${n.toFixed(2).replace(".", ",")} €`;
}

function docProgress(r: RegistrationRow): string | null {
  const blocking = r.documents.filter((d) => d.kind === "avant_signature");
  if (blocking.length === 0) return null;
  const done = blocking.filter((d) => d.uploaded || d.exempt).length;
  return `${done}/${blocking.length} pièces`;
}

export function TripRegistrationsPanel({ tripId }: { tripId: string }) {
  const [data, setData] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [filter, setFilter] = useState<FollowupFilter | "all">("all");
  const [paperFor, setPaperFor] = useState<RegistrationRow | null>(null);

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

  /** Relance ciblée : aperçu des familles, puis envoi SEULEMENT après confirmation explicite de l'utilisateur. */
  const remind = async (target: { invitationId: string } | { status: FollowupFilter }) => {
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const res = await fetch("/api/travels/v2/invitations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, action: "remind_preview", ...target }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Aperçu impossible");
      const families = (j.families || []) as { studentName: string; email: string | null }[];
      if (families.length === 0) {
        setMsg("Aucune famille à relancer pour cette cible.");
        return;
      }
      const names = families.slice(0, 12).map((f) => `• ${f.studentName}`).join("\n");
      const more = families.length > 12 ? `\n… et ${families.length - 12} autre(s)` : "";
      if (!window.confirm(`Envoyer une relance par e-mail à ${families.length} famille(s) ?\n\n${names}${more}`)) return;
      const send = await fetch("/api/travels/v2/invitations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId, action: "remind_send", confirm: true, ...target }),
      });
      const sj = await send.json().catch(() => ({}));
      if (!send.ok) throw new Error(sj.error || "Envoi impossible");
      setMsg(`Relance : ${sj.sent} envoyée(s)${sj.failed ? `, ${sj.failed} en échec` : ""}${sj.skipped ? `, ${sj.skipped} déjà relancée(s) aujourd'hui ou page fermée` : ""}.`);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Relance impossible");
    } finally {
      setBusy(false);
    }
  };

  const decide = async (r: RegistrationRow, decision: "maintenir" | "retirer") => {
    const label = decision === "maintenir" ? "MAINTENIR l'inscription de" : "RETIRER du voyage";
    if (!window.confirm(`Confirmer : ${label} ${r.studentName} ?\nLa décision est enregistrée dans l'historique.`)) return;
    const note = window.prompt("Note (facultatif, conservée dans l'historique) :", "") || undefined;
    await act({ action: "decide", ine: r.ine, decision, note }, decision === "maintenir" ? "Inscription maintenue." : "Élève retiré du voyage.");
  };

  if (error && !data) return <TripAlert tone="warning" title="Inscriptions">{error}</TripAlert>;
  if (!data) return <p className="text-sm text-slate-500">Chargement des inscriptions…</p>;
  if (!data.opened) return null;

  const rows = data.rows.filter((r) => rowMatchesFilter(r, filter));
  const open = data.status === "INSCRIPTIONS_OUVERTES";
  const count = (k: string) => data.counts[k] || 0;
  const neverSent = data.rows.filter((r) => r.invitationId && !r.sentAt).length;
  const exportHref = (format: "csv" | "xlsx") => `/api/travels/v2/invitations/export?tripId=${encodeURIComponent(tripId)}&format=${format}`;

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
        {data.suspended > 0 ? (
          <TripAlert tone="warning" title={`${data.suspended} élève(s) en suspens (désaccord entre responsables)`}>
            {data.isDirection
              ? "Ces élèves ne sont pas comptés dans la liste officielle : maintenez l'inscription ou retirez l'élève (boutons sur la ligne)."
              : "Ces élèves ne sont pas comptés dans la liste officielle tant que la direction n'a pas tranché."}
          </TripAlert>
        ) : null}
        {data.withoutEmail > 0 ? (
          <TripAlert tone="warning" title={`${data.withoutEmail} élève(s) de la pré-liste sans e-mail de responsable`}>
            Aucun lien ne peut leur être envoyé : complétez la fiche élève puis « Actualiser avec les fiches élèves ». Un coupon papier reste possible.
          </TripAlert>
        ) : null}

        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          <div className="rounded-xl border bg-white p-3">
            <p className="text-xs text-slate-500">Liste officielle (inscrits)</p>
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
          <select value={filter} onChange={(e) => setFilter(e.target.value as FollowupFilter | "all")} className="rounded-lg border bg-white p-2 text-sm" aria-label="Filtrer par statut">
            <option value="all">Tous les statuts</option>
            {(Object.keys(FILTER_LABELS) as FollowupFilter[]).map((k) => (
              <option key={k} value={k}>
                {FILTER_LABELS[k]}
              </option>
            ))}
          </select>
          {data.canWrite && open && REMIND_FILTERS.includes(filter as FollowupFilter) ? (
            <TripButton size="sm" variant="secondary" disabled={busy || !data.smtpOk} onClick={() => void remind({ status: filter as FollowupFilter })}>
              Relancer toutes les familles « {FILTER_LABELS[filter as FollowupFilter]} »
            </TripButton>
          ) : null}
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
          <a href={exportHref("csv")} className="rounded-lg border bg-white px-3 py-1.5 text-xs font-semibold text-slate-700 hover:bg-slate-50">
            Export CSV
          </a>
          <a href={exportHref("xlsx")} className="rounded-lg border bg-white px-3 py-1.5 text-xs font-semibold text-slate-700 hover:bg-slate-50">
            Export Excel
          </a>
          <TripButton size="sm" variant="secondary" disabled={busy} onClick={() => void load()}>
            Rafraîchir
          </TripButton>
        </div>
        {open ? (
          <p className="text-xs text-slate-500">
            Rappels automatiques : 3 jours et 1 jour avant la date limite pour les familles sans réponse ; pièces « avant le départ » relancées jusqu&apos;à leur date, et alerte
            à l&apos;organisateur 7 jours avant le départ s&apos;il en manque.
          </p>
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
                    {r.status === "en_cours" && docProgress(r) ? <span className="ml-1 text-[11px] text-slate-500">{docProgress(r)}</span> : null}
                    {r.paper ? <span className="ml-1 rounded-full bg-violet-100 px-2 py-0.5 text-[11px] font-bold text-violet-800">Papier</span> : null}
                    {r.exempted && !r.signed ? <span className="ml-1 rounded-full bg-teal-100 px-2 py-0.5 text-[11px] font-bold text-teal-800">Dispensé</span> : null}
                    {r.status === "bloque_piece" ? (
                      <span className="mt-1 block text-[11px] text-rose-700">
                        Manque : {r.documents.filter((d) => d.kind === "avant_signature" && !d.uploaded && !d.exempt).map((d) => d.label).join(", ")}
                      </span>
                    ) : null}
                    {r.objectionBy ? (
                      <span className="mt-1 block text-[11px] text-red-700">
                        Désaccord de {r.objectionBy}
                        {r.objectionReason ? ` : « ${r.objectionReason} »` : ""}
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
                    <div className="flex flex-col items-start gap-1">
                      {data.isDirection && (r.status === "suspens" || r.status === "desaccord") ? (
                        <>
                          <button type="button" className="text-xs font-bold text-emerald-700 underline" disabled={busy} onClick={() => void decide(r, "maintenir")}>
                            Maintenir l&apos;inscription
                          </button>
                          <button type="button" className="text-xs font-bold text-red-700 underline" disabled={busy} onClick={() => void decide(r, "retirer")}>
                            Retirer l&apos;élève
                          </button>
                        </>
                      ) : null}
                      {data.canWrite && open && !r.signedAt && r.status !== "retire" ? (
                        <button type="button" className="text-xs font-semibold text-violet-700 underline" disabled={busy} onClick={() => setPaperFor(r)}>
                          Enregistrer un coupon papier
                        </button>
                      ) : null}
                      {r.invitationId ? (
                        <>
                          {r.signed && r.official ? (
                            <button type="button" className="text-xs font-semibold text-indigo-600 underline" onClick={() => void download(r.invitationId!, { engagement: true })}>
                              {r.paper ? "Coupon papier (scan)" : "Engagement (PDF)"}
                            </button>
                          ) : null}
                          {data.canWrite && open && !r.signedAt ? (
                            <>
                              <button type="button" className="text-xs font-semibold text-indigo-600 underline" disabled={busy} onClick={() => void act({ action: "resend", invitationId: r.invitationId }, "Lien renvoyé.")}>
                                Renvoyer le lien
                              </button>
                              <button type="button" className="text-xs font-semibold text-indigo-600 underline" disabled={busy || !data.smtpOk} onClick={() => void remind({ invitationId: r.invitationId! })}>
                                Relancer cette famille
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
                            </>
                          ) : null}
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
                  </td>
                </tr>
              ))}
              {rows.length === 0 ? (
                <tr>
                  <td colSpan={6} className="py-6 text-center text-sm text-slate-500">
                    Aucune ligne pour ce filtre.
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
      </div>
      {paperFor ? (
        <PaperDialog
          tripId={tripId}
          row={paperFor}
          picnicDays={data.picnicDays}
          onClose={() => setPaperFor(null)}
          onDone={() => {
            setPaperFor(null);
            setMsg("Coupon papier enregistré : il compte comme une signature.");
            void load();
          }}
        />
      ) : null}
    </TripSection>
  );
}

/** Coupon papier : réponse (oui / non, paniers) saisie par l'organisateur, avec dépôt du scan (obligatoire). */
function PaperDialog({
  tripId,
  row,
  picnicDays,
  onClose,
  onDone,
}: {
  tripId: string;
  row: RegistrationRow;
  picnicDays: { key: string; label: string }[];
  onClose: () => void;
  onDone: () => void;
}) {
  const [decision, setDecision] = useState<"oui" | "non">("oui");
  const [signerName, setSignerName] = useState("");
  const [phone, setPhone] = useState("");
  const [meals, setMeals] = useState<Record<string, { wants: "oui" | "non"; type?: MealType }>>({});
  const [file, setFile] = useState<File | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    setErr(null);
    if (!file) return setErr("Déposez le scan du coupon.");
    if (signerName.trim().length < 2) return setErr("Indiquez le nom du signataire.");
    if (decision === "oui" && picnicDays.some((d) => !meals[d.key])) return setErr("Renseignez chaque jour de panier repas.");
    setBusy(true);
    try {
      const fd = new FormData();
      fd.set(
        "meta",
        JSON.stringify({
          tripId,
          invitationId: row.invitationId,
          decision,
          signerName: signerName.trim(),
          ...(phone.trim() ? { phone: phone.trim() } : {}),
          ...(decision === "oui" && picnicDays.length ? { meals } : {}),
        }),
      );
      fd.set("scan", file);
      const res = await fetch("/api/travels/v2/invitations/paper", { method: "POST", body: fd });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      onDone();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Enregistrement impossible");
    } finally {
      setBusy(false);
    }
  };

  const inputCls = "rounded-lg border bg-white p-2 text-sm";
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4" role="dialog" aria-modal="true" aria-label="Coupon papier">
      <div className="max-h-[90vh] w-full max-w-lg space-y-3 overflow-y-auto rounded-2xl bg-white p-5 shadow-xl">
        <h3 className="text-base font-bold">Coupon papier — {row.studentName}</h3>
        <p className="text-xs text-slate-600">
          La réponse est enregistrée avec la mention « papier » et compte comme une signature (non modifiable ensuite). Le scan du coupon signé est conservé comme engagement.
        </p>
        {err ? <p role="alert" className="rounded-lg bg-rose-50 p-2 text-sm font-semibold text-rose-700">{err}</p> : null}
        <fieldset className="space-y-1">
          <legend className="text-sm font-bold">Réponse de la famille</legend>
          <div className="flex gap-4 text-sm">
            {(["oui", "non"] as const).map((v) => (
              <label key={v} className="flex items-center gap-2">
                <input type="radio" name="paper-decision" checked={decision === v} onChange={() => setDecision(v)} />
                {v === "oui" ? "Participe" : "Ne participe pas"}
              </label>
            ))}
          </div>
        </fieldset>
        <label className="block text-sm">
          <span className="font-semibold">Nom du signataire (parent)</span>
          <input className={`${inputCls} mt-1 w-full`} value={signerName} onChange={(e) => setSignerName(e.target.value)} maxLength={100} />
        </label>
        {decision === "oui" ? (
          <>
            <label className="block text-sm">
              <span className="font-semibold">Téléphone pendant le voyage (facultatif)</span>
              <input className={`${inputCls} mt-1 w-full`} value={phone} onChange={(e) => setPhone(e.target.value)} maxLength={20} />
            </label>
            {picnicDays.length > 0 ? (
              <fieldset className="space-y-2">
                <legend className="text-sm font-bold">Paniers repas</legend>
                {picnicDays.map((d) => {
                  const c = meals[d.key];
                  return (
                    <div key={d.key} className="flex flex-wrap items-center gap-3 rounded-lg border p-2 text-sm">
                      <span className="w-28 font-semibold">{d.label}</span>
                      {(["oui", "non"] as const).map((v) => (
                        <label key={v} className="flex items-center gap-1">
                          <input
                            type="radio"
                            name={`paper-${d.key}`}
                            checked={c?.wants === v}
                            onChange={() => setMeals((m) => ({ ...m, [d.key]: v === "oui" ? { wants: "oui", type: m[d.key]?.type ?? "classique" } : { wants: "non" } }))}
                          />
                          {v === "oui" ? "Oui" : "Non (repas famille)"}
                        </label>
                      ))}
                      {c?.wants === "oui" ? (
                        <select className={inputCls} value={c.type ?? "classique"} aria-label={`Type de panier — ${d.label}`} onChange={(e) => setMeals((m) => ({ ...m, [d.key]: { wants: "oui", type: e.target.value as MealType } }))}>
                          {MEAL_TYPE_VALUES.map((t) => (
                            <option key={t} value={t}>
                              {MEAL_TYPE_LABELS[t]}
                            </option>
                          ))}
                        </select>
                      ) : null}
                    </div>
                  );
                })}
              </fieldset>
            ) : null}
          </>
        ) : null}
        <label className="block text-sm">
          <span className="font-semibold">Scan du coupon signé (PDF ou image, 8 Mo max.)</span>
          <input type="file" accept="application/pdf,image/jpeg,image/png,image/webp" className="mt-1 block w-full text-sm" onChange={(e) => setFile(e.target.files?.[0] ?? null)} />
        </label>
        <div className="flex justify-end gap-2 pt-2">
          <TripButton size="sm" variant="secondary" disabled={busy} onClick={onClose}>
            Annuler
          </TripButton>
          <TripButton size="sm" disabled={busy} onClick={() => void submit()}>
            {busy ? "Enregistrement…" : "Enregistrer la réponse papier"}
          </TripButton>
        </div>
      </div>
    </div>
  );
}
