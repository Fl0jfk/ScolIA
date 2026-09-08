"use client";

import Image from "next/image";
import { useState } from "react";
import type { MailPreviewType } from "@/app/lib/travels-mail-preview";
import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { normalizeTravelImageUrl } from "@/app/lib/travels-image-url";
import { TripButton, TripSection } from "@/app/components/travels/TripDetailUI";
import { TripMailPreviewModal } from "@/app/components/travels/hub/TripMailPreviewModal";

function existingTransportRequest(trip: TravelsTrip) {
  const tr = trip.data.transportRequest;
  if (!tr || typeof tr !== "object") {
    return { pickupPoint: "", stayOnSite: false, freeText: "" };
  }
  return {
    pickupPoint: typeof tr.pickupPoint === "string" ? tr.pickupPoint : "",
    stayOnSite: Boolean(tr.stayOnSite),
    freeText: typeof tr.freeText === "string" ? tr.freeText : "",
  };
}

export function TripActionsPanel({
  trip,
  canManage,
  isGlobalAdmin = false,
  onTripUpdated,
}: {
  trip: TravelsTrip;
  canManage: boolean;
  isGlobalAdmin?: boolean;
  onTripUpdated: (trip: TravelsTrip) => void;
}) {
  const [busy, setBusy] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ type: MailPreviewType; label: string } | null>(null);
  const [cancelNotifyTransport, setCancelNotifyTransport] = useState(true);
  const [cancelNotifyCuisine, setCancelNotifyCuisine] = useState(true);
  const [showRequalifyModal, setShowRequalifyModal] = useState(false);
  const [requalifyForm, setRequalifyForm] = useState(() => existingTransportRequest(trip));

  const canRequalifyToBus =
    canManage &&
    !complexNeedsBus(trip) &&
    (trip.type === "SIMPLE" || trip.type === "COMPLEX") &&
    !["ANNULE", "SEANCE_ANNULEE", "REJETE"].includes(String(trip.status));

  const coverUrl = normalizeTravelImageUrl(
    (typeof trip.imageUrl === "string" && trip.imageUrl) ||
      (typeof trip.data?.imageUrl === "string" ? trip.data.imageUrl : undefined),
  );

  const exportZip = async () => {
    setBusy("zip");
    try {
      const res = await fetch("/api/travels/export-zip", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId: trip.id }),
      });
      if (!res.ok) {
        const j = await res.json().catch(() => ({}));
        throw new Error(j.error || "Export impossible");
      }
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `Dossier_${trip.id}.zip`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur export");
    } finally {
      setBusy(null);
    }
  };

  const regenerateCoverImage = async () => {
    if (!confirm("Régénérer l'image de présentation de ce séjour avec l'IA ?")) return;
    setBusy("regen-image");
    try {
      const res = await fetch("/api/travels/regenerate-image", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tripId: trip.id }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Régénération impossible");
      if (j.trip) onTripUpdated(j.trip);
      alert(
        j.imageLabel
          ? `Nouvelle image : ${j.imageLabel}`
          : "Image de présentation mise à jour.",
      );
    } catch (e) {
      alert(e instanceof Error ? e.message : "Régénération impossible");
    } finally {
      setBusy(null);
    }
  };

  const cancelTrip = async () => {
    const reason = prompt("Motif d'annulation (optionnel) :") ?? "";
    if (!confirm("Confirmer l'annulation de cette sortie ? Les prestataires peuvent être notifiés.")) return;
    setBusy("cancel");
    try {
      const res = await fetch("/api/travels/cancel-trip", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: trip.id,
          reason,
          notifyTransport: cancelNotifyTransport,
          notifyCuisine: cancelNotifyCuisine,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error);
      onTripUpdated(j.trip);
      alert(
        j.emailsSent?.length
          ? `Sortie annulée. ${j.emailsSent.length} notification(s) envoyée(s).`
          : "Sortie annulée.",
      );
    } catch (e) {
      alert(e instanceof Error ? e.message : "Annulation impossible");
    } finally {
      setBusy(null);
    }
  };

  const openRequalifyModal = () => {
    setRequalifyForm(existingTransportRequest(trip));
    setShowRequalifyModal(true);
  };

  const confirmRequalifyToBus = async () => {
    setBusy("requalify");
    try {
      const res = await fetch("/api/travels/requalify-to-bus", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tripId: trip.id,
          transportRequest: {
            pickupPoint: requalifyForm.pickupPoint.trim(),
            stayOnSite: requalifyForm.stayOnSite,
            freeText: requalifyForm.freeText.trim(),
          },
        }),
      });
      const j = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(j.error || "Requalification impossible");
      if (j.trip) onTripUpdated(j.trip);
      setShowRequalifyModal(false);
      const failed = Number(j.emailsFailed) || 0;
      const attempted = Number(j.emailsAttempted) || 0;
      alert(
        failed > 0
          ? `Dossier requalifié en voyage / sortie bus. Demande de devis : ${attempted - failed}/${attempted} envoi(s) OK.`
          : `Dossier requalifié en voyage / sortie bus. Demande de devis envoyée aux transporteurs${attempted ? ` (${attempted})` : ""}.`,
      );
    } catch (e) {
      alert(e instanceof Error ? e.message : "Requalification impossible");
    } finally {
      setBusy(null);
    }
  };

  const previewOptions: { type: MailPreviewType; label: string; show: boolean }[] = [
    { type: "transport_amendment", label: "Avenant transport", show: complexNeedsBus(trip) },
    { type: "cuisine_amendment", label: "Annule et remplace cuisine", show: Boolean(trip.data.cuisineOrderSentAt) },
    { type: "cuisine_initial", label: "Commande cuisine initiale", show: Boolean(trip.data.piqueNiqueDetails?.active) },
    { type: "cancel_trip_transport", label: "Annulation transport", show: complexNeedsBus(trip) },
    { type: "cancel_trip_cuisine", label: "Annulation cuisine", show: Boolean(trip.data.cuisineOrderSentAt) },
  ];

  return (
    <>
      <TripSection title="Actions du sas voyage" subtitle="Export, annulation, aperçus mails" icon="⚡">
        <div className="grid gap-4 sm:grid-cols-2">
          {canRequalifyToBus && (
            <div className="rounded-xl border border-amber-200 bg-amber-50/60 p-4 space-y-3 sm:col-span-2">
              <h3 className="font-bold text-amber-950 text-sm">
                {trip.type === "SIMPLE"
                  ? "Requalifier en voyage / sortie bus"
                  : "Activer le transport bus + devis"}
              </h3>
              <p className="text-xs text-amber-900/80">
                {trip.type === "SIMPLE"
                  ? "Cette sortie de proximité devient un voyage scolaire avec autocar : le circuit devis transporteur est ouvert et une demande de devis part immédiatement."
                  : "Ce voyage était sans bus. Active l’autocar, ouvre l’onglet Transport et envoie la demande de devis aux transporteurs."}
              </p>
              <TripButton variant="primary" size="sm" disabled={!!busy} onClick={openRequalifyModal}>
                {busy === "requalify" ? "Requalification…" : "Passer en sortie bus (avec devis)"}
              </TripButton>
            </div>
          )}

          <div className="rounded-xl border border-slate-200 p-4 space-y-3">
            <h3 className="font-bold text-slate-800 text-sm">Export & archivage</h3>
            <p className="text-xs text-slate-500">
              Télécharge un ZIP avec pièces jointes, devis bus et manifeste JSON du dossier.
            </p>
            <TripButton variant="primary" size="sm" disabled={!!busy} onClick={exportZip}>
              {busy === "zip" ? "Préparation…" : "Exporter le dossier (ZIP)"}
            </TripButton>
          </div>

          <div className="rounded-xl border border-slate-200 p-4 space-y-3">
            <h3 className="font-bold text-slate-800 text-sm">Aperçu des mails</h3>
            <p className="text-xs text-slate-500">Prévisualiser le contenu avant envoi (sans envoyer).</p>
            <div className="flex flex-wrap gap-2">
              {previewOptions
                .filter((o) => o.show)
                .map((o) => (
                  <button
                    key={o.type}
                    type="button"
                    onClick={() => setPreview(o)}
                    className="text-xs font-bold text-indigo-600 hover:underline"
                  >
                    {o.label}
                  </button>
                ))}
            </div>
          </div>

          {isGlobalAdmin && (
            <div className="rounded-xl border border-slate-200 p-4 space-y-3 sm:col-span-2">
              <h3 className="font-bold text-slate-800 text-sm">Image de présentation</h3>
              <p className="text-xs text-slate-500">
                Réservé aux administrateurs. Relance la sélection IA dans le catalogue d’images à partir
                du titre et de la destination.
              </p>
              <div className="flex flex-col sm:flex-row sm:items-center gap-4">
                <div className="relative h-28 w-full sm:w-44 shrink-0 overflow-hidden rounded-lg bg-slate-200">
                  {coverUrl ? (
                    <Image
                      src={coverUrl}
                      alt={trip.data?.title || "Image de présentation"}
                      fill
                      className="object-cover"
                      sizes="176px"
                    />
                  ) : (
                    <div className="flex h-full items-center justify-center text-3xl">🎒</div>
                  )}
                </div>
                <TripButton
                  variant="primary"
                  size="sm"
                  disabled={!!busy}
                  onClick={regenerateCoverImage}
                >
                  {busy === "regen-image" ? "Régénération…" : "Régénérer l’image (IA)"}
                </TripButton>
              </div>
            </div>
          )}

          {canManage && !["ANNULE", "SEANCE_ANNULEE", "REJETE"].includes(trip.status) && (
            <div className="rounded-xl border border-red-200 bg-red-50/50 p-4 space-y-3 sm:col-span-2">
              <h3 className="font-bold text-red-900 text-sm">Annuler la sortie</h3>
              <p className="text-xs text-red-800/80">
                Passe le dossier en statut « Sortie annulée » et notifie optionnellement transport et cuisine.
              </p>
              <div className="flex flex-wrap gap-4 text-xs">
                {complexNeedsBus(trip) && (
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={cancelNotifyTransport}
                      onChange={(e) => setCancelNotifyTransport(e.target.checked)}
                    />
                    Prévenir le transporteur
                  </label>
                )}
                {trip.data.cuisineOrderSentAt && (
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={cancelNotifyCuisine}
                      onChange={(e) => setCancelNotifyCuisine(e.target.checked)}
                    />
                    Prévenir le chef (cuisine)
                  </label>
                )}
              </div>
              <TripButton variant="danger" size="sm" disabled={!!busy} onClick={cancelTrip}>
                {busy === "cancel" ? "Annulation…" : "Annuler cette sortie"}
              </TripButton>
            </div>
          )}
        </div>
      </TripSection>

      {preview && (
        <TripMailPreviewModal
          tripId={trip.id}
          type={preview.type}
          label={preview.label}
          onClose={() => setPreview(null)}
        />
      )}

      {showRequalifyModal && (
        <div className="fixed inset-0 z-[70] flex items-center justify-center bg-slate-900/60 p-4 backdrop-blur-sm">
          <div className="w-full max-w-lg rounded-3xl border border-amber-100 bg-white p-6 shadow-2xl sm:p-8">
            <div className="mb-5 flex items-start gap-3">
              <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-amber-100 text-xl">
                🚌
              </div>
              <div>
                <h2 className="text-lg font-black text-slate-900">Passer en sortie bus</h2>
                <p className="text-sm text-slate-500">
                  Une demande de devis sera envoyée à tous les transporteurs configurés.
                </p>
              </div>
            </div>
            <div className="mb-6 space-y-4 rounded-2xl border border-slate-100 bg-slate-50 p-4">
              <div>
                <label className="mb-1 block text-xs font-bold text-slate-600">
                  Lieu de prise en charge (RDV)
                </label>
                <input
                  className="w-full rounded-xl border bg-white p-3 text-sm outline-amber-500"
                  placeholder="Ex. : Devant le gymnase"
                  value={requalifyForm.pickupPoint}
                  onChange={(e) =>
                    setRequalifyForm((prev) => ({ ...prev, pickupPoint: e.target.value }))
                  }
                />
              </div>
              <label className="flex items-center gap-3 text-xs font-bold text-slate-600">
                <input
                  type="checkbox"
                  className="h-5 w-5"
                  checked={requalifyForm.stayOnSite}
                  onChange={(e) =>
                    setRequalifyForm((prev) => ({ ...prev, stayOnSite: e.target.checked }))
                  }
                />
                Le bus reste sur place pour les visites
              </label>
              <div>
                <label className="mb-1 block text-xs font-bold text-slate-600">
                  Informations complémentaires
                </label>
                <textarea
                  rows={2}
                  className="w-full rounded-xl border bg-white p-3 text-sm outline-amber-500"
                  placeholder="Précisions pour le transporteur…"
                  value={requalifyForm.freeText}
                  onChange={(e) =>
                    setRequalifyForm((prev) => ({ ...prev, freeText: e.target.value }))
                  }
                />
              </div>
            </div>
            <div className="flex gap-3">
              <button
                type="button"
                disabled={!!busy}
                onClick={() => setShowRequalifyModal(false)}
                className="flex-1 rounded-2xl bg-slate-100 py-3 text-sm font-black text-slate-600"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={!!busy}
                onClick={() => void confirmRequalifyToBus()}
                className="flex-[2] rounded-2xl bg-amber-500 py-3 text-sm font-black text-white shadow-lg disabled:opacity-50"
              >
                {busy === "requalify" ? "Envoi…" : "Valider et envoyer le devis"}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
