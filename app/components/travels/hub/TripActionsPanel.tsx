"use client";

import Image from "next/image";
import { useState } from "react";
import type { MailPreviewType } from "@/app/lib/travels-mail-preview";
import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { normalizeTravelImageUrl } from "@/app/lib/travels-image-url";
import { TripButton, TripSection } from "@/app/components/travels/TripDetailUI";
import { TripMailPreviewModal } from "@/app/components/travels/hub/TripMailPreviewModal";

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
    </>
  );
}
