"use client";

import type { Dispatch, FormEvent, SetStateAction } from "react";
import { busLogisticsActive } from "@/app/lib/travels-trip-helpers";
import { orderEmailForQuote } from "@/app/lib/travels-transport-shared";
import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  TripAlert,
  TripBusQuoteCard,
  TripButton,
  TripInput,
  TripSection,
  TripTextarea,
} from "@/app/components/travels/TripDetailUI";

type TripTransportHubPanelProps = {
  trip: TravelsTrip;
  setTrip: Dispatch<SetStateAction<TravelsTrip | null>>;
  loadingAction: string | null;
  canRequestAmendedQuote: boolean;
  requestAmendedBusQuote: () => void;
  canSign: boolean;
  skipTransportToCompta: () => void | Promise<void>;
  openSecureFile: (url: string, key?: string | null) => void;
  effectifChanged: boolean;
  datesChanged: boolean;
  snapshotEffectifTotal: number | null;
  currentEffectifTotal: number;
  isOwner: boolean;
  selectBusQuote: (quote: Record<string, unknown>) => void;
  selectAndSignBusQuote: (quote: Record<string, unknown>) => void;
  deleteBusQuote: (quote: Record<string, unknown>) => void;
  canAddDocuments: boolean;
  addManualBusQuote: (e: FormEvent<HTMLFormElement>) => void;
  manualDevisName: string;
  setManualDevisName: (v: string) => void;
  manualDevisEmail: string;
  setManualDevisEmail: (v: string) => void;
  manualDevisBusy: boolean;
  signBusQuote: () => void;
  handleAction: (status: string, note?: string, extra?: Record<string, unknown>) => void;
  transportReplyTo: string;
  setTransportReplyTo: (v: string) => void;
  transportReplyBody: string;
  setTransportReplyBody: (v: string) => void;
  transportReplyBusy: boolean;
  setTransportReplyBusy: (v: boolean) => void;
};

export function TripTransportHubPanel({
  trip,
  setTrip,
  loadingAction,
  canRequestAmendedQuote,
  requestAmendedBusQuote,
  canSign,
  skipTransportToCompta,
  openSecureFile,
  effectifChanged,
  datesChanged,
  snapshotEffectifTotal,
  currentEffectifTotal,
  isOwner,
  selectBusQuote,
  selectAndSignBusQuote,
  deleteBusQuote,
  canAddDocuments,
  addManualBusQuote,
  manualDevisName,
  setManualDevisName,
  manualDevisEmail,
  setManualDevisEmail,
  manualDevisBusy,
  signBusQuote,
  handleAction,
  transportReplyTo,
  setTransportReplyTo,
  transportReplyBody,
  setTransportReplyBody,
  transportReplyBusy,
  setTransportReplyBusy,
}: TripTransportHubPanelProps) {
  return (
        <TripSection
          title="Devis transport"
          subtitle="Offres reçues par e-mail ou ajoutées manuellement"
          icon="🚌"
          accent="amber"
          action={
            canRequestAmendedQuote ? (
              <TripButton
                variant="warning"
                size="sm"
                onClick={() => requestAmendedBusQuote()}
                disabled={loadingAction === "amendment-quote"}
              >
                {loadingAction === "amendment-quote" ? "Envoi…" : "Devis rectifié (effectif)"}
              </TripButton>
            ) : undefined
          }
        >
          {trip.data.transportPhaseBypassedAt && (
            <TripAlert tone="info" title="Étape transport contournée" icon="ℹ️">
              <p className="text-xs leading-relaxed">
                La direction a validé le passage aux finances sans devis bus signé
                {trip.data.transportPhaseBypassedBy ? ` (${trip.data.transportPhaseBypassedBy})` : ""}.
                {trip.data.transportPhaseBypassNote ? (
                  <>
                    <br />
                    <span className="italic">{trip.data.transportPhaseBypassNote}</span>
                  </>
                ) : null}
              </p>
            </TripAlert>
          )}
          {trip.data.pendingAmendedQuote && trip.status === "PROF_LOGISTICS" && (
            <TripAlert tone="warning" title="Devis rectifié en attente" icon="⏳">
              <p className="text-xs leading-relaxed">
                Une demande de devis (avenant) a été envoyée aux transporteurs. Si le devis n&apos;est plus
                nécessaire, la direction peut passer aux finances sans attendre.
              </p>
              {canSign && (
                <TripButton
                  variant="warning"
                  size="sm"
                  className="mt-3"
                  onClick={() => void skipTransportToCompta()}
                  disabled={!!loadingAction}
                >
                  Passer aux finances sans devis signé
                </TripButton>
              )}
            </TripAlert>
          )}
          {trip.data.transportProviderConfirmation && (
            <TripAlert tone="success" title="Confirmation transporteur reçue" icon="✅">
              <p className="text-xs leading-relaxed">{trip.data.transportProviderConfirmation.summary}</p>
              {trip.data.transportProviderConfirmation.receivedAt && (
                <p className="text-[10px] text-emerald-800/80 mt-1">
                  {new Date(trip.data.transportProviderConfirmation.receivedAt).toLocaleString("fr-FR")}
                  {trip.data.transportProviderConfirmation.providerName
                    ? ` · ${trip.data.transportProviderConfirmation.providerName}`
                    : ""}
                </p>
              )}
              {trip.data.transportProviderConfirmation.pdfUrl && (
                <TripButton
                  variant="ghost"
                  size="sm"
                  type="button"
                  className="mt-2"
                  onClick={() =>
                    openSecureFile(
                      trip.data.transportProviderConfirmation!.pdfUrl!,
                      trip.data.transportProviderConfirmation!.s3KeyIncoming,
                    )
                  }
                >
                  Voir le PDF de confirmation
                </TripButton>
              )}
            </TripAlert>
          )}
          {(effectifChanged || datesChanged) && (
            <TripAlert tone="warning" title="Dossier modifié depuis la dernière demande transport">
              <span className="text-xs">
                Dernier envoi : {snapshotEffectifTotal} pers. · Actuel : {currentEffectifTotal} pers.
                {trip.data?.selectedBusQuote
                  ? ` → avenant vers ${trip.data.selectedBusQuote.providerName} uniquement.`
                  : " → avenant vers tous les transporteurs."}
              </span>
            </TripAlert>
          )}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-4">
            <div className="space-y-3">
              <p className="text-[10px] font-bold uppercase tracking-wider text-amber-800">Offres reçues</p>
              {trip.receivedDevis && trip.receivedDevis.length > 0 ? (
                trip.receivedDevis.map((quote: any, idx: number) => {
                  const reviewBus =
                    quote.matchReviewRequired === true ||
                    (quote.source === "email" &&
                      quote.matchConfidence &&
                      quote.matchConfidence !== "high");
                  const borderSelected = trip.data.selectedBusQuote?.fileUrl === quote.fileUrl;
                  return (
                  <TripBusQuoteCard
                    key={quote.id || idx}
                    selected={borderSelected}
                    review={reviewBus}
                    actions={
                      <>
                        <div className="flex gap-2">
                          <TripButton
                            variant="ghost"
                            size="sm"
                            type="button"
                            onClick={() => openSecureFile(quote.fileUrl, quote.s3KeyIncoming)}
                          >
                            Voir PDF
                          </TripButton>
                          {isOwner && !canSign && trip.status === "PROF_LOGISTICS" && (
                            <TripButton variant="primary" size="sm" onClick={() => selectBusQuote(quote)}>
                              Choisir
                            </TripButton>
                          )}
                          {canSign && trip.status === "PROF_LOGISTICS" && (
                            <TripButton
                              variant="success"
                              size="sm"
                              onClick={() => selectAndSignBusQuote(quote)}
                              disabled={!!loadingAction}
                            >
                              Choisir et signer
                            </TripButton>
                          )}
                        </div>
                        {canSign && (
                          <button
                            type="button"
                            onClick={() => deleteBusQuote(quote)}
                            disabled={!!loadingAction}
                            className="text-[10px] font-bold text-rose-600 hover:text-rose-800 disabled:opacity-50 text-center"
                          >
                            {loadingAction === `delete-quote-${quote.id}` ? "Suppression…" : "Supprimer"}
                          </button>
                        )}
                      </>
                    }
                  >
                      <p className="font-bold text-slate-900">{quote.providerName}</p>
                      <p className="text-indigo-600 font-semibold text-xs mt-0.5">
                        Devis reçu
                        {quote.source === "email" ? " · e-mail" : quote.source === "manual" ? " · manuel" : ""}
                      </p>
                      {reviewBus && (
                        <p className="mt-1 text-[10px] font-bold text-orange-800 uppercase tracking-wide">
                          À vérifier — rattachement automatique ({quote.matchConfidence || "?"})
                        </p>
                      )}
                      {quote.matchMotif && reviewBus && (
                        <p className="mt-0.5 text-[10px] text-orange-900/90 leading-snug">{quote.matchMotif}</p>
                      )}
                      {(quote.extractedPrice || quote.extractedCompany) && (
                        <p className="mt-1 text-[11px] text-slate-600">
                          {quote.extractedCompany ? <span className="font-medium">{quote.extractedCompany}</span> : null}
                          {quote.extractedCompany && quote.extractedPrice ? " · " : null}
                          {quote.extractedPrice ? <span className="font-semibold text-slate-800">{quote.extractedPrice}</span> : null}
                        </p>
                      )}
                      {(() => {
                        const to = orderEmailForQuote(quote);
                        return to ? (
                          <p className="mt-1.5 text-[10px] text-slate-700">
                            <span className="font-bold text-slate-500">Commande →</span>{" "}
                            <span className="font-mono">{to}</span>
                            {quote.extractedContactEmail?.trim() ? (
                              <span className="text-emerald-700 font-semibold"> (sur le devis)</span>
                            ) : quote.providerEmail?.trim() ? (
                              <span className="text-slate-500">
                                {" "}
                                ({quote.source === "manual" ? "saisi à la main" : "expéditeur mail"})
                              </span>
                            ) : null}
                          </p>
                        ) : (
                          <p className="mt-1.5 text-[10px] font-bold text-rose-700">
                            Aucun e-mail pour la commande — renseigner ou vérifier le devis.
                          </p>
                        );
                      })()}
                  </TripBusQuoteCard>
                  );
                })
              ) : (
                <p className="text-sm text-slate-400 italic py-4 text-center rounded-xl border border-dashed border-amber-200">
                  En attente de devis par e-mail…
                </p>
              )}
              {canAddDocuments && (
                <form onSubmit={addManualBusQuote} className="mt-4 p-4 rounded-xl border border-dashed border-amber-300 bg-amber-50/50 space-y-3 text-left">
                  <p className="text-xs font-bold text-amber-900 uppercase tracking-wide">Ajout manuel</p>
                  <p className="text-[11px] text-amber-900/70 leading-snug">
                    Déposez le PDF et l&apos;e-mail du transporteur si le devis n&apos;arrive pas par la boîte dédiée.
                  </p>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <label className="block text-[11px] font-semibold text-slate-600">
                      Transporteur
                      <TripInput
                        className="mt-1"
                        value={manualDevisName}
                        onChange={(ev) => setManualDevisName(ev.target.value)}
                        placeholder="ex. Cars Dupont"
                        disabled={manualDevisBusy}
                      />
                    </label>
                    <label className="block text-[11px] font-semibold text-slate-600">
                      E-mail (commande)
                      <TripInput
                        type="email"
                        required
                        className="mt-1"
                        value={manualDevisEmail}
                        onChange={(ev) => setManualDevisEmail(ev.target.value)}
                        placeholder="contact@transporteur.fr"
                        disabled={manualDevisBusy}
                      />
                    </label>
                  </div>
                  <label className="block text-[11px] font-semibold text-slate-600">
                    PDF du devis
                    <input
                      name="manualDevisPdf"
                      type="file"
                      accept="application/pdf"
                      className="mt-1 block w-full text-sm text-slate-600 file:mr-3 file:py-1.5 file:px-3 file:rounded-lg file:border-0 file:bg-amber-100 file:font-semibold file:text-amber-900"
                      disabled={manualDevisBusy}
                    />
                  </label>
                  <TripButton type="submit" variant="warning" size="sm" disabled={manualDevisBusy}>
                    {manualDevisBusy ? "Envoi…" : "Ajouter ce devis"}
                  </TripButton>
                </form>
              )}
            </div>
            <div className="rounded-xl border border-amber-200 bg-white p-6 flex flex-col justify-center min-h-[12rem]">
              {trip.data.selectedBusQuote ? (
                <div className="text-center space-y-4">
                  <div>
                    <p className="text-[10px] font-bold uppercase tracking-wider text-amber-700">Devis retenu</p>
                    <p className="text-lg font-bold text-slate-900 mt-1">{String(trip.data.selectedBusQuote.providerName ?? "")}</p>
                    {orderEmailForQuote(trip.data.selectedBusQuote) && (
                      <p className="text-xs text-slate-500 mt-1 font-mono">{orderEmailForQuote(trip.data.selectedBusQuote)}</p>
                    )}
                  </div>
                  {canSign && trip.status === "EN_ATTENTE_BUS_SIGNATURE" && (
                    <div className="flex flex-col gap-3 w-full">
                      <TripButton
                        variant="success"
                        size="lg"
                        onClick={() => signBusQuote()}
                        disabled={!!loadingAction}
                        className="w-full"
                      >
                        ✍️ Signer et commander
                      </TripButton>
                      <TripButton
                        variant="warning"
                        size="sm"
                        onClick={() => void skipTransportToCompta()}
                        disabled={!!loadingAction}
                        className="w-full"
                      >
                        Passer aux finances sans signer
                      </TripButton>
                      <button
                        type="button"
                        onClick={() => { const n = prompt("Pourquoi refusez-vous ce devis ?"); if (n) handleAction("PROF_LOGISTICS", n); }}
                        className="text-xs font-bold text-rose-600 hover:underline"
                      >
                        Refuser ce choix
                      </button>
                    </div>
                  )}
                  {isOwner && !canSign && trip.status === "EN_ATTENTE_BUS_SIGNATURE" && (
                    <p className="text-sm text-amber-800 bg-amber-50 border border-amber-200 rounded-xl px-4 py-3">
                      Devis choisi — en attente de signature par la direction.
                    </p>
                  )}
                  {(trip.status === "EN_ATTENTE_COMPTA" || trip.status === "EN_ATTENTE_DIR_FINAL" || trip.status === "VALIDE") && (
                    <p className="inline-flex items-center gap-2 text-emerald-700 font-bold text-sm bg-emerald-50 px-4 py-2 rounded-full">
                      ✓ Commandé et signé
                    </p>
                  )}
                </div>
              ) : (
                <p className="text-sm text-slate-400 italic text-center">
                  Le créateur choisira un devis à l&apos;étape « Choix du devis transport ».
                </p>
              )}
            </div>
          </div>
          {((Array.isArray(trip.data.transportEmailMessages) && trip.data.transportEmailMessages.length > 0) ||
            busLogisticsActive(trip)) && (
            <div className="mt-6 pt-6 border-t border-amber-200/80">
              <p className="text-[10px] font-bold uppercase tracking-wider text-amber-800 mb-3">
                Messagerie transporteur (e-mail)
              </p>
              {Array.isArray(trip.data.transportEmailMessages) && trip.data.transportEmailMessages.length > 0 && (
              <div className="space-y-3">
                {trip.data.transportEmailMessages.map((msg: {
                  id: string;
                  messageType?: string;
                  summary?: string;
                  subject?: string;
                  fromEmail?: string;
                  toEmail?: string;
                  direction?: string;
                  driverName?: string | null;
                  driverPhone?: string | null;
                  details?: string | null;
                  pdfUrl?: string | null;
                  s3KeyIncoming?: string | null;
                  receivedAt?: string;
                  matchConfidence?: string | null;
                }) => {
                  const isOut = msg.direction === "outbound";
                  const typeLabel = isOut
                    ? "Envoyé"
                    : msg.messageType === "confirmation_commande"
                      ? "Confirmation"
                      : msg.messageType === "info_transport"
                        ? "Info transport"
                        : msg.messageType === "devis_pdf"
                          ? "Devis"
                          : "Message reçu";
                  return (
                    <div
                      key={msg.id}
                      className={`rounded-xl border px-4 py-3 text-sm ${
                        isOut
                          ? "border-indigo-100 bg-indigo-50/50 text-slate-800"
                          : "border-amber-100 bg-amber-50/40 text-slate-800"
                      }`}
                    >
                      <div className="flex flex-wrap items-center gap-2 mb-1">
                        <span
                          className={`text-[10px] font-bold uppercase tracking-wide px-2 py-0.5 rounded-full ${
                            isOut ? "text-indigo-800 bg-indigo-100" : "text-amber-800 bg-amber-100"
                          }`}
                        >
                          {typeLabel}
                        </span>
                        {msg.receivedAt && (
                          <span className="text-[10px] text-slate-500">
                            {new Date(msg.receivedAt).toLocaleString("fr-FR")}
                          </span>
                        )}
                        {!isOut && msg.matchConfidence && msg.matchConfidence !== "high" && (
                          <span className="text-[10px] text-amber-700 font-semibold">À vérifier</span>
                        )}
                      </div>
                      {msg.summary && <p className="leading-snug whitespace-pre-wrap">{msg.summary}</p>}
                      {(msg.driverName || msg.driverPhone) && (
                        <p className="mt-2 text-xs text-slate-700">
                          {msg.driverName && <span className="font-semibold">Chauffeur : {msg.driverName}</span>}
                          {msg.driverName && msg.driverPhone ? " · " : null}
                          {msg.driverPhone && <span className="font-mono">{msg.driverPhone}</span>}
                        </p>
                      )}
                      {msg.details && msg.details !== msg.summary && (
                        <p className="mt-2 text-xs text-slate-600 whitespace-pre-wrap">{msg.details}</p>
                      )}
                      {msg.pdfUrl && (
                        <TripButton
                          variant="ghost"
                          size="sm"
                          type="button"
                          className="mt-2"
                          onClick={() => openSecureFile(msg.pdfUrl!, msg.s3KeyIncoming)}
                        >
                          Voir la pièce jointe PDF
                        </TripButton>
                      )}
                      <p className="mt-2 text-[10px] text-slate-500 truncate" title={msg.subject}>
                        {isOut
                          ? `À ${msg.toEmail || "—"} — ${msg.subject || ""}`
                          : `${msg.fromEmail} — ${msg.subject || ""}`}
                      </p>
                      {!isOut && msg.fromEmail && (
                        <TripButton
                          variant="ghost"
                          size="sm"
                          type="button"
                          className="mt-2"
                          onClick={() => {
                            setTransportReplyTo(msg.fromEmail || "");
                            setTransportReplyBody("");
                          }}
                        >
                          Répondre à {msg.fromEmail}
                        </TripButton>
                      )}
                    </div>
                  );
                })}
              </div>
              )}
              <div className="mt-4 rounded-xl border border-slate-200 bg-white/80 p-4 space-y-3">
                <p className="text-xs font-bold text-slate-800">Répondre au transporteur</p>
                <TripInput
                  type="email"
                  placeholder="Adresse e-mail du transporteur"
                  value={transportReplyTo}
                  onChange={(e) => setTransportReplyTo(e.target.value)}
                />
                <TripTextarea
                  rows={4}
                  placeholder="Votre message…"
                  value={transportReplyBody}
                  onChange={(e) => setTransportReplyBody(e.target.value)}
                />
                <TripButton
                  type="button"
                  disabled={transportReplyBusy || !transportReplyTo.trim() || transportReplyBody.trim().length < 2}
                  onClick={async () => {
                    if (!trip?.id) return;
                    setTransportReplyBusy(true);
                    try {
                      const res = await fetch("/api/travels/reply-transport-email", {
                        method: "POST",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({
                          tripId: trip.id,
                          toEmail: transportReplyTo.trim(),
                          bodyText: transportReplyBody.trim(),
                        }),
                      });
                      const payload = await res.json().catch(() => ({}));
                      if (!res.ok) {
                        alert(payload.error || "Échec de l'envoi");
                        return;
                      }
                      if (payload.trip) setTrip(payload.trip);
                      setTransportReplyBody("");
                    } catch {
                      alert("Erreur réseau");
                    } finally {
                      setTransportReplyBusy(false);
                    }
                  }}
                >
                  {transportReplyBusy ? "Envoi…" : "Envoyer l'e-mail"}
                </TripButton>
              </div>
            </div>
          )}
        </TripSection>

  );
}
