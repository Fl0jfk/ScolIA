"use client";

import type { Dispatch, SetStateAction } from "react";
import TripClassesMultiSelect from "@/app/components/travels/TripClassesMultiSelect";
import { emptyCuisineDetails, getTotalMeals } from "@/app/lib/travels-cuisine-form";
import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  TripField,
  TripFieldActions,
  TripFieldValue,
  TripInput,
  TripSection,
  TripTextarea,
} from "@/app/components/travels/TripDetailUI";

type TripOverviewFieldsPanelProps = {
  trip: TravelsTrip;
  isEditing: boolean;
  editedData: Record<string, unknown> & {
    classes?: string;
    nomsAccompagnateurs?: string;
    nbEleves?: string | number;
    nbAccompagnateurs?: string | number;
    startDate?: string;
    date?: string;
    endDate?: string;
    startTime?: string;
    endTime?: string;
    coutTotal?: number;
    objectifs?: string;
    piqueNiqueDetails?: ReturnType<typeof emptyCuisineDetails>;
  };
  setEditedData: Dispatch<SetStateAction<any>>;
  classOptions: string[];
  canEditEffectif: boolean;
  openEffectifModal: () => void;
  withBusLogistics: boolean;
  effectifChanged: boolean;
  cuisineOrderSent: boolean;
  cuisineChanged: boolean;
  loadingAction: string | null;
  requestAmendedBusQuote: () => void;
  sendCuisineAmendment: () => void;
  dateLabel: string;
  canEditDates: boolean;
  datesChanged: boolean;
  openDateModal: () => void;
  canAccessComptaTab: boolean;
  setHubTab: (tab: "compta" | "cuisine" | "documents") => void;
  openBudgetModal: () => void;
  openCuisineModalFromEdit: () => void;
  openCuisineModalForOwner: () => void;
  documentCount: number;
};

export function TripOverviewFieldsPanel(p: TripOverviewFieldsPanelProps) {
  const {
    trip, isEditing, editedData, setEditedData, classOptions, canEditEffectif, openEffectifModal,
    withBusLogistics, effectifChanged, cuisineOrderSent, cuisineChanged, loadingAction,
    requestAmendedBusQuote, sendCuisineAmendment, dateLabel, canEditDates, datesChanged,
    openDateModal, canAccessComptaTab, setHubTab, openBudgetModal, openCuisineModalFromEdit,
    openCuisineModalForOwner, documentCount,
  } = p;
  return (
      <TripSection title="Détails du dossier" subtitle="Informations logistiques et pédagogiques" icon="📋">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-6">
          <TripField label="Destination" span={2}>
            <TripFieldValue value={trip.data.destination} multiline />
          </TripField>
          <TripField label="Classes concernées">
            {isEditing ? (
              <TripClassesMultiSelect
                value={String(editedData.classes || "")}
                options={classOptions}
                onChange={(classes) => setEditedData({ ...editedData, classes })}
              />
            ) : (
              <TripFieldValue value={trip.data.classes} />
            )}
          </TripField>
          <TripField label="Accompagnateurs">
            {isEditing ? (
              <TripInput value={editedData.nomsAccompagnateurs} onChange={(e) => setEditedData({ ...editedData, nomsAccompagnateurs: e.target.value })} />
            ) : (
              <>
                <TripFieldValue value={trip.data.nomsAccompagnateurs || "—"} />
                {canEditEffectif && (
                  <TripFieldActions>
                    <button
                      type="button"
                      onClick={openEffectifModal}
                      className="text-xs font-bold text-indigo-600 hover:underline"
                    >
                      Modifier effectifs &amp; accompagnateurs
                    </button>
                  </TripFieldActions>
                )}
              </>
            )}
          </TripField>
          <TripField label="Effectifs">
            {isEditing ? (
              <div className="flex gap-3">
                <div className="flex-1">
                  <span className="text-[9px] text-slate-400">Élèves</span>
                  <TripInput type="number" value={editedData.nbEleves} onChange={(e) => setEditedData({ ...editedData, nbEleves: e.target.value })} />
                </div>
                <div className="flex-1">
                  <span className="text-[9px] text-slate-400">Accomp.</span>
                  <TripInput type="number" value={editedData.nbAccompagnateurs} onChange={(e) => setEditedData({ ...editedData, nbAccompagnateurs: Number(e.target.value) })} />
                </div>
              </div>
            ) : (
              <>
                <TripFieldValue value={`${trip.data.nbEleves} élèves · ${trip.data.nbAccompagnateurs || "0"} accompagnateurs`} />
                {(canEditEffectif ||
                  (withBusLogistics && effectifChanged) ||
                  (cuisineOrderSent && trip.data?.piqueNiqueDetails?.active && cuisineChanged)) && (
                  <TripFieldActions>
                    {canEditEffectif && (
                      <button
                        type="button"
                        onClick={openEffectifModal}
                        className="text-xs font-bold text-indigo-600 hover:underline"
                      >
                        Modifier l&apos;effectif
                      </button>
                    )}
                    {withBusLogistics && effectifChanged && (
                      <button
                        type="button"
                        onClick={() => requestAmendedBusQuote()}
                        disabled={loadingAction === "amendment-quote"}
                        className="text-xs font-bold text-amber-700 hover:underline disabled:opacity-50"
                      >
                        Demander un devis rectifié (transport)
                      </button>
                    )}
                    {cuisineOrderSent && Boolean(trip.data?.piqueNiqueDetails?.active) && cuisineChanged && (
                      <button
                        type="button"
                        onClick={() => sendCuisineAmendment()}
                        disabled={loadingAction === "cuisine-amendment"}
                        className="text-xs font-bold text-emerald-700 hover:underline disabled:opacity-50"
                      >
                        Renvoyer commande cuisine (annule et remplace)
                      </button>
                    )}
                  </TripFieldActions>
                )}
              </>
            )}
          </TripField>
          <TripField label="Dates">
            {isEditing ? (
              <div className="flex gap-2 flex-wrap">
                <TripInput type="date" value={editedData.startDate || editedData.date || ""} onChange={(e) => setEditedData({ ...editedData, startDate: e.target.value, date: e.target.value })} />
                {trip.type === "COMPLEX" && (
                  <TripInput type="date" value={editedData.endDate || ""} onChange={(e) => setEditedData({ ...editedData, endDate: e.target.value })} />
                )}
              </div>
            ) : (
              <>
                <TripFieldValue value={dateLabel} />
                {(canEditDates || datesChanged) && (
                  <TripFieldActions>
                    {canEditDates && (
                      <button type="button" onClick={openDateModal} className="text-xs font-bold text-indigo-600 hover:underline">
                        Modifier dates & horaires
                      </button>
                    )}
                    {datesChanged && (
                      <p className="text-[10px] text-amber-700 font-semibold">Dates modifiées depuis le dernier envoi transport</p>
                    )}
                  </TripFieldActions>
                )}
              </>
            )}
          </TripField>
          <TripField label="Horaires">
            {isEditing ? (
              <div className="flex gap-2">
                <TripInput placeholder="Départ" value={editedData.startTime} onChange={(e) => setEditedData({ ...editedData, startTime: e.target.value })} />
                <TripInput placeholder="Retour" value={editedData.endTime} onChange={(e) => setEditedData({ ...editedData, endTime: e.target.value })} />
              </div>
            ) : (
              <TripFieldValue value={`Départ ${trip.data.startTime || "—"} · Retour ${trip.data.endTime || "—"}`} />
            )}
          </TripField>
          <TripField label="Budget">
            {isEditing ? (
              <div className="flex items-center gap-2">
                <TripInput type="number" className="max-w-[8rem]" value={editedData.coutTotal} onChange={(e) => setEditedData({ ...editedData, coutTotal: Number(e.target.value) })} />
                <span className="text-xs font-bold text-slate-500">€ total</span>
              </div>
            ) : (
              <div>
                <TripFieldValue value={`${Math.round(Number(trip.data.coutTotal))} € prévisionnel`} />
                {trip.data.finalTotalCost && (
                  <p className="text-emerald-700 font-bold text-sm mt-1">
                    Validé compta : {trip.data.finalTotalCost} € ({trip.data.costPerStudent} €/élève)
                  </p>
                )}
                {canAccessComptaTab && (
                  <button
                    type="button"
                    onClick={() => setHubTab("compta")}
                    className="text-xs font-bold text-indigo-600 hover:underline mt-1 block"
                  >
                    Ouvrir l&apos;onglet Compta
                  </button>
                )}
                {canEditEffectif && (
                  <TripFieldActions>
                    <button
                      type="button"
                      onClick={openBudgetModal}
                      className="text-xs font-bold text-indigo-600 hover:underline"
                    >
                      Modifier le budget prévisionnel
                    </button>
                  </TripFieldActions>
                )}
              </div>
            )}
          </TripField>
          <TripField label="Restauration">
            {isEditing ? (
              <button
                type="button"
                onClick={openCuisineModalFromEdit}
                className={`w-full p-4 rounded-xl border-2 flex items-center justify-between transition-all text-left ${
                  editedData?.piqueNiqueDetails?.active ? "border-emerald-400 bg-emerald-50" : "border-slate-200 bg-slate-50"
                }`}
              >
                <div>
                  <p className="font-bold text-slate-900 text-sm">Commande restauration</p>
                  <p className="text-xs text-slate-500 mt-0.5">
                    {editedData?.piqueNiqueDetails?.active
                      ? `${getTotalMeals(editedData.piqueNiqueDetails)} repas configurés`
                      : "Configurer"}
                  </p>
                </div>
                <span className="text-xl">🥪</span>
              </button>
            ) : (
              <div>
                <TripFieldValue value={Boolean(trip.data.piqueNiqueDetails?.active) ? "Commande cuisine configurée" : "Pas de commande cuisine"} />
                {Boolean(trip.data.piqueNiqueDetails?.active) && (
                  <>
                    <span className="inline-block mt-1 text-[10px] font-bold text-indigo-700 bg-indigo-50 px-2 py-0.5 rounded-full">
                      {getTotalMeals(trip.data.piqueNiqueDetails ?? emptyCuisineDetails())} repas ·{" "}
                      {Object.values(trip.data.piqueNiqueDetails?.daysSelection || {}).filter(Boolean).length} jour(s)
                    </span>
                    <button
                      type="button"
                      onClick={() => setHubTab("cuisine")}
                      className="mt-2 block text-xs font-bold text-emerald-700 hover:underline"
                    >
                      Voir le détail restauration →
                    </button>
                  </>
                )}
                {canEditEffectif && (
                  <TripFieldActions>
                    <button
                      type="button"
                      onClick={openCuisineModalForOwner}
                      className="text-xs font-bold text-indigo-600 hover:underline"
                    >
                      {trip.data.piqueNiqueDetails?.active
                        ? "Modifier la commande cuisine"
                        : "Configurer une commande cuisine"}
                    </button>
                  </TripFieldActions>
                )}
              </div>
            )}
          </TripField>
          <TripField label="Objectifs pédagogiques" span={2}>
            {isEditing ? (
              <TripTextarea value={editedData.objectifs} onChange={(e) => setEditedData({ ...editedData, objectifs: e.target.value })} />
            ) : (
              <TripFieldValue value={trip.data.objectifs || "Aucun objectif renseigné."} multiline />
            )}
          </TripField>
        </div>
        {documentCount > 0 && (
          <p className="mt-6 text-xs text-slate-500">
            {documentCount} document{documentCount > 1 ? "s" : ""} dans le dossier —{" "}
            <button type="button" onClick={() => setHubTab("documents")} className="font-bold text-indigo-600 hover:underline">
              voir l&apos;onglet Documents
            </button>
          </p>
        )}
      </TripSection>

  );
}
