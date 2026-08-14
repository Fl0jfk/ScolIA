"use client";

import type { TravelsTrip } from "@/app/lib/travels-types";
import { TripButton, TripDecisionPanel } from "@/app/components/travels/TripDetailUI";

type TripDecisionHubPanelProps = {
  trip: TravelsTrip;
  isDirection: boolean;
  canSign: boolean;
  etabForSign: string;
  isCompta: boolean;
  handleAction: (status: string, note?: string, extra?: Record<string, unknown>) => void;
  withBusLogistics: boolean;
  isOwner: boolean;
  skipTransportToCompta: () => void | Promise<void>;
  loadingAction: string | null;
  seriesId: string | null | undefined;
  validateSeriesPedagogy: () => void;
  handleFinalValidation: () => void;
  handleRegenerateCircular: () => void;
  reopenStepOptions: { value: string; label: string }[];
  selectedReopenStep: string;
  setReopenStep: (v: string) => void;
  handleReopenDossier: (value: string, label: string) => void;
  canCancelRecurrenceSession: boolean;
  cancelRecurrenceSession: () => void;
};

export function TripDecisionHubPanel(p: TripDecisionHubPanelProps) {
  const {
    trip, isDirection, canSign, etabForSign, isCompta, handleAction, withBusLogistics,
    isOwner, skipTransportToCompta, loadingAction, seriesId, validateSeriesPedagogy,
    handleFinalValidation, handleRegenerateCircular, reopenStepOptions, selectedReopenStep,
    setReopenStep, handleReopenDossier, canCancelRecurrenceSession, cancelRecurrenceSession,
  } = p;
  return (
        <TripDecisionPanel title="Espace décisionnaire">
          <div className="flex flex-wrap gap-2 items-center lg:flex-1">
            {isDirection && !canSign && (
              <div className="w-full sm:max-w-md rounded-xl bg-white/5 border border-white/10 px-4 py-3 text-sm text-slate-300">
                <p className="font-bold text-white text-xs uppercase tracking-wide mb-1">Lecture seule</p>
                Dossier <span className="text-amber-300 font-semibold">{etabForSign || "groupe scolaire"}</span> — validation réservée à la direction concernée.
              </div>
            )}
          </div>
          <div className="flex flex-wrap gap-2 justify-end">
            {((canSign && (trip.status === "EN_ATTENTE_DIR_INITIAL" || trip.status === "EN_ATTENTE_BUS_SIGNATURE" || trip.status === "EN_ATTENTE_DIR_FINAL")) || (isCompta && trip.status === "EN_ATTENTE_COMPTA")) && (
              <>
                {canSign && (
                  <TripButton variant="danger" size="sm" onClick={() => { const n = prompt("Motif du refus définitif :"); if (n) handleAction("REJETE", n); }}>
                    Refus définitif
                  </TripButton>
                )}
                <TripButton
                  variant="warning"
                  size="sm"
                  onClick={() => {
                    const n = prompt("Précisez les changements attendus :");
                    if (n) {
                      const returnTo = trip.status === "EN_ATTENTE_DIR_FINAL" ? "EN_ATTENTE_COMPTA" : trip.status;
                      handleAction("BESOIN_MODIFICATION", n, { previousStatus: returnTo });
                    }
                  }}
                >
                  Demander des modifs
                </TripButton>
              </>
            )}
            {canSign && trip.status === "EN_ATTENTE_DIR_INITIAL" && trip.type === "COMPLEX" && (
              <TripButton
                variant="primary"
                size="sm"
                onClick={() =>
                  handleAction(
                    withBusLogistics ? "PROF_LOGISTICS" : "EN_ATTENTE_COMPTA",
                    withBusLogistics ? "Pédagogie validée" : "Pédagogie validée (sans transport bus)",
                  )
                }
              >
                Valider pédagogie
              </TripButton>
            )}
            {(isOwner || canSign) && trip.type === "COMPLEX" && !withBusLogistics && trip.status === "PROF_LOGISTICS" && (
              <TripButton variant="primary" size="sm" onClick={() => handleAction("EN_ATTENTE_COMPTA", "Sans bus — étape logistique non requise")}>
                Passer aux finances
              </TripButton>
            )}
            {canSign &&
              withBusLogistics &&
              (trip.status === "PROF_LOGISTICS" || trip.status === "EN_ATTENTE_BUS_SIGNATURE") && (
                <TripButton variant="warning" size="sm" onClick={() => void skipTransportToCompta()} disabled={!!loadingAction}>
                  Passer aux finances sans devis signé
                </TripButton>
              )}
            {canSign && trip.status === "EN_ATTENTE_DIR_INITIAL" && trip.type !== "COMPLEX" && !seriesId && (
              <TripButton variant="primary" size="sm" onClick={() => handleAction("EN_ATTENTE_COMPTA", "Pédagogie validée")}>
                Valider pédagogie
              </TripButton>
            )}
            {canSign && trip.status === "EN_ATTENTE_DIR_INITIAL" && trip.type !== "COMPLEX" && seriesId && (
              <TripButton variant="primary" size="sm" onClick={validateSeriesPedagogy}>
                {loadingAction ? "Validation série…" : "Valider toute la série"}
              </TripButton>
            )}
            {canSign && trip.status === "EN_ATTENTE_DIR_FINAL" && (
              <TripButton variant="success" size="sm" onClick={handleFinalValidation}>
                {loadingAction ? "Finalisation…" : "Validation finale"}
              </TripButton>
            )}
            {trip.status === "VALIDE" && (canSign || isOwner) && (
              <TripButton variant="success" size="sm" onClick={handleRegenerateCircular} disabled={!!loadingAction}>
                {loadingAction === "regenerate-circular" ? "Génération…" : "Régénérer circulaire"}
              </TripButton>
            )}
            {canSign && trip.status === "VALIDE" && reopenStepOptions.length > 0 && (
              <div className="flex flex-wrap items-center gap-2 rounded-xl bg-white/5 border border-white/10 px-3 py-2">
                <label htmlFor="reopen-step-select" className="text-xs text-slate-300 shrink-0">
                  Réouvrir :
                </label>
                <select
                  id="reopen-step-select"
                  value={selectedReopenStep}
                  onChange={(e) => setReopenStep(e.target.value)}
                  className="bg-slate-800 text-white text-sm font-medium rounded-lg px-2 py-1.5 border border-slate-600 outline-none focus:ring-2 focus:ring-indigo-400 min-w-[8rem]"
                >
                  {reopenStepOptions.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
                <TripButton
                  variant="dark"
                  size="sm"
                  onClick={() => {
                    const opt = reopenStepOptions.find((o) => o.value === selectedReopenStep);
                    if (opt) handleReopenDossier(opt.value, opt.label);
                  }}
                >
                  {loadingAction ? "…" : "Réouvrir"}
                </TripButton>
              </div>
            )}
            {canCancelRecurrenceSession && canSign && (
              <TripButton variant="secondary" size="sm" onClick={cancelRecurrenceSession}>
                Annuler cette séance
              </TripButton>
            )}
          </div>
        </TripDecisionPanel>

  );
}
