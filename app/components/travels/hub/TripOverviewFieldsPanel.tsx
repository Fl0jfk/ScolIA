"use client";

import type { Dispatch, SetStateAction, ReactNode } from "react";
import TripClassesMultiSelect from "@/app/components/travels/TripClassesMultiSelect";
import TripAccompagnateursSelect, {
  accompagnateursToFormFields,
  formFieldsToAccompagnateurs,
} from "@/app/components/travels/TripAccompagnateursSelect";
import { emptyCuisineDetails, getTotalMeals } from "@/app/lib/travels-cuisine-form";
import type { TravelsAccompagnateur } from "@/app/lib/travels-accompagnateurs";
import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  TripFieldActions,
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
    accompagnateurs?: TravelsAccompagnateur[];
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

function InfoTile({
  icon,
  label,
  tone = "slate",
  children,
  action,
  span = 1,
}: {
  icon: string;
  label: string;
  tone?: "slate" | "indigo" | "emerald" | "amber" | "sky";
  children: ReactNode;
  action?: ReactNode;
  span?: 1 | 2;
}) {
  const tones = {
    slate: "border-slate-200/90 bg-white",
    indigo: "border-indigo-100 bg-gradient-to-br from-indigo-50/70 to-white",
    emerald: "border-emerald-100 bg-gradient-to-br from-emerald-50/70 to-white",
    amber: "border-amber-100 bg-gradient-to-br from-amber-50/70 to-white",
    sky: "border-sky-100 bg-gradient-to-br from-sky-50/70 to-white",
  };
  const iconBg = {
    slate: "bg-slate-100",
    indigo: "bg-indigo-100",
    emerald: "bg-emerald-100",
    amber: "bg-amber-100",
    sky: "bg-sky-100",
  };
  return (
    <div
      className={`rounded-2xl border p-4 shadow-sm ${tones[tone]} ${span === 2 ? "sm:col-span-2" : ""}`}
    >
      <div className="flex items-center gap-2.5 mb-2.5">
        <span
          className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl text-base ${iconBg[tone]}`}
        >
          {icon}
        </span>
        <p className="text-[10px] font-bold uppercase tracking-wider text-slate-500">{label}</p>
      </div>
      <div className="text-sm text-slate-800 font-medium leading-snug">{children}</div>
      {action}
    </div>
  );
}

function EmptyValue() {
  return <span className="text-slate-400 italic font-normal">—</span>;
}

export function TripOverviewFieldsPanel(p: TripOverviewFieldsPanelProps) {
  const {
    trip,
    isEditing,
    editedData,
    setEditedData,
    classOptions,
    canEditEffectif,
    openEffectifModal,
    withBusLogistics,
    effectifChanged,
    cuisineOrderSent,
    cuisineChanged,
    loadingAction,
    requestAmendedBusQuote,
    sendCuisineAmendment,
    dateLabel,
    canEditDates,
    datesChanged,
    openDateModal,
    canAccessComptaTab,
    setHubTab,
    openBudgetModal,
    openCuisineModalFromEdit,
    openCuisineModalForOwner,
    documentCount,
  } = p;

  const cuisineActive = Boolean(trip.data.piqueNiqueDetails?.active);
  const mealCount = cuisineActive
    ? getTotalMeals(trip.data.piqueNiqueDetails ?? emptyCuisineDetails())
    : 0;
  const cuisineDays = cuisineActive
    ? Object.values(trip.data.piqueNiqueDetails?.daysSelection || {}).filter(Boolean).length
    : 0;

  const accompagnateursLabel = trip.data.nomsAccompagnateurs
    ? `${trip.data.nbAccompagnateurs || 0} — ${trip.data.nomsAccompagnateurs}`
    : `${trip.data.nbAccompagnateurs || 0} accompagnateur(s)${
        Number(trip.data.nbAccompagnateurs || 0) > 0 ? " (noms à préciser)" : ""
      }`;

  return (
    <TripSection
      title="Détails du dossier"
      subtitle="Repères rapides — logistique et pédagogie"
      icon="📋"
    >
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <InfoTile icon="📍" label="Destination" tone="indigo" span={2}>
          {trip.data.destination ? (
            <span className="whitespace-pre-wrap leading-relaxed text-base font-bold text-slate-900">
              {trip.data.destination}
            </span>
          ) : (
            <EmptyValue />
          )}
        </InfoTile>

        <InfoTile icon="🏫" label="Classes concernées" tone="sky">
          {isEditing ? (
            <TripClassesMultiSelect
              value={String(editedData.classes || "")}
              options={classOptions}
              onChange={(classes) => setEditedData({ ...editedData, classes })}
            />
          ) : trip.data.classes ? (
            <span className="font-bold text-slate-900">{trip.data.classes}</span>
          ) : (
            <EmptyValue />
          )}
        </InfoTile>

        <InfoTile
          icon="👥"
          label="Effectifs"
          tone="indigo"
          action={
            !isEditing &&
            (canEditEffectif ||
              (withBusLogistics && effectifChanged) ||
              (cuisineOrderSent && cuisineActive && cuisineChanged)) ? (
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
                {cuisineOrderSent && cuisineActive && cuisineChanged && (
                  <button
                    type="button"
                    onClick={() => sendCuisineAmendment()}
                    disabled={loadingAction === "cuisine-amendment"}
                    className="text-xs font-bold text-emerald-700 hover:underline disabled:opacity-50"
                  >
                    Renvoyer commande cuisine
                  </button>
                )}
              </TripFieldActions>
            ) : undefined
          }
        >
          {isEditing ? (
            <div className="flex gap-3">
              <div className="flex-1">
                <span className="text-[9px] text-slate-400">Élèves</span>
                <TripInput
                  type="number"
                  value={editedData.nbEleves}
                  onChange={(e) => setEditedData({ ...editedData, nbEleves: e.target.value })}
                />
              </div>
              <div className="flex-1">
                <span className="text-[9px] text-slate-400">Accomp.</span>
                <TripInput
                  type="number"
                  min={0}
                  value={editedData.nbAccompagnateurs}
                  title="Nombre déclaré pour le transport — les noms peuvent être ajoutés ensuite"
                  onChange={(e) => {
                    const raw = e.target.value;
                    const named = formFieldsToAccompagnateurs({
                      nomsAccompagnateurs: String(editedData.nomsAccompagnateurs || ""),
                      accompagnateurs: editedData.accompagnateurs,
                    }).length;
                    const n = Number(raw);
                    const safe =
                      Number.isFinite(n) && n >= 0 ? Math.max(Math.floor(n), named) : named;
                    setEditedData({ ...editedData, nbAccompagnateurs: safe });
                  }}
                />
              </div>
            </div>
          ) : (
            <p className="text-base font-bold text-slate-900">
              {trip.data.nbEleves} élèves
              <span className="text-slate-400 font-semibold mx-1.5">·</span>
              {trip.data.nbAccompagnateurs || "0"} accompagnateurs
            </p>
          )}
        </InfoTile>

        <InfoTile
          icon="🧑‍🏫"
          label="Accompagnateurs"
          tone="slate"
          span={2}
          action={
            !isEditing && canEditEffectif ? (
              <TripFieldActions>
                <button
                  type="button"
                  onClick={openEffectifModal}
                  className="text-xs font-bold text-indigo-600 hover:underline"
                >
                  Modifier effectifs &amp; accompagnateurs
                </button>
              </TripFieldActions>
            ) : undefined
          }
        >
          {isEditing ? (
            <div className="space-y-3">
              <div>
                <span className="text-[9px] text-slate-400">Nombre (noms optionnels)</span>
                <TripInput
                  type="number"
                  min={0}
                  value={editedData.nbAccompagnateurs}
                  onChange={(e) => {
                    const raw = e.target.value;
                    const named = formFieldsToAccompagnateurs({
                      nomsAccompagnateurs: String(editedData.nomsAccompagnateurs || ""),
                      accompagnateurs: editedData.accompagnateurs,
                    }).length;
                    const n = Number(raw);
                    const safe =
                      Number.isFinite(n) && n >= 0 ? Math.max(Math.floor(n), named) : named;
                    setEditedData({ ...editedData, nbAccompagnateurs: safe });
                  }}
                />
              </div>
              <TripAccompagnateursSelect
                value={formFieldsToAccompagnateurs({
                  nomsAccompagnateurs: String(editedData.nomsAccompagnateurs || ""),
                  accompagnateurs: editedData.accompagnateurs,
                })}
                onChange={(accompagnateurs) =>
                  setEditedData({
                    ...editedData,
                    ...accompagnateursToFormFields(accompagnateurs, {
                      declaredNb: editedData.nbAccompagnateurs,
                    }),
                  })
                }
              />
            </div>
          ) : (
            <span className="leading-relaxed">{accompagnateursLabel}</span>
          )}
        </InfoTile>

        <InfoTile
          icon="📅"
          label="Dates"
          tone="amber"
          action={
            !isEditing && (canEditDates || datesChanged) ? (
              <TripFieldActions>
                {canEditDates && (
                  <button
                    type="button"
                    onClick={openDateModal}
                    className="text-xs font-bold text-indigo-600 hover:underline"
                  >
                    Modifier dates &amp; horaires
                  </button>
                )}
                {datesChanged && (
                  <p className="text-[10px] text-amber-700 font-semibold">
                    Dates modifiées depuis le dernier envoi transport
                  </p>
                )}
              </TripFieldActions>
            ) : undefined
          }
        >
          {isEditing ? (
            <div className="flex gap-2 flex-wrap">
              <TripInput
                type="date"
                value={editedData.startDate || editedData.date || ""}
                onChange={(e) =>
                  setEditedData({
                    ...editedData,
                    startDate: e.target.value,
                    date: e.target.value,
                  })
                }
              />
              {trip.type === "COMPLEX" && (
                <TripInput
                  type="date"
                  value={editedData.endDate || ""}
                  onChange={(e) => setEditedData({ ...editedData, endDate: e.target.value })}
                />
              )}
            </div>
          ) : (
            <span className="font-bold text-slate-900">{dateLabel || <EmptyValue />}</span>
          )}
        </InfoTile>

        <InfoTile icon="🕐" label="Horaires" tone="amber">
          {isEditing ? (
            <div className="flex gap-2">
              <TripInput
                placeholder="Départ"
                value={editedData.startTime}
                onChange={(e) => setEditedData({ ...editedData, startTime: e.target.value })}
              />
              <TripInput
                placeholder="Retour"
                value={editedData.endTime}
                onChange={(e) => setEditedData({ ...editedData, endTime: e.target.value })}
              />
            </div>
          ) : (
            <span>
              Départ <strong>{trip.data.startTime || "—"}</strong>
              <span className="text-slate-400 mx-1.5">·</span>
              Retour <strong>{trip.data.endTime || "—"}</strong>
            </span>
          )}
        </InfoTile>

        <InfoTile
          icon="💶"
          label="Budget"
          tone="emerald"
          action={
            !isEditing ? (
              <div className="mt-3 flex flex-col items-start gap-1.5">
                {canAccessComptaTab && (
                  <button
                    type="button"
                    onClick={() => setHubTab("compta")}
                    className="text-xs font-bold text-indigo-600 hover:underline"
                  >
                    Ouvrir l&apos;onglet Compta
                  </button>
                )}
                {canEditEffectif && (
                  <button
                    type="button"
                    onClick={openBudgetModal}
                    className="text-xs font-bold text-indigo-600 hover:underline"
                  >
                    Modifier le budget prévisionnel
                  </button>
                )}
              </div>
            ) : undefined
          }
        >
          {isEditing ? (
            <div className="flex items-center gap-2">
              <TripInput
                type="number"
                className="max-w-[8rem]"
                value={editedData.coutTotal}
                onChange={(e) =>
                  setEditedData({ ...editedData, coutTotal: Number(e.target.value) })
                }
              />
              <span className="text-xs font-bold text-slate-500">€ total</span>
            </div>
          ) : (
            <div>
              <p className="text-base font-bold text-slate-900">
                {Math.round(Number(trip.data.coutTotal))} €{" "}
                <span className="text-xs font-semibold text-slate-500">prévisionnel</span>
              </p>
              {trip.data.finalTotalCost && (
                <p className="text-emerald-700 font-bold text-sm mt-1">
                  Validé compta : {trip.data.finalTotalCost} € ({trip.data.costPerStudent} €/élève)
                </p>
              )}
            </div>
          )}
        </InfoTile>

        <InfoTile
          icon="🥪"
          label="Restauration"
          tone={cuisineActive || editedData?.piqueNiqueDetails?.active ? "emerald" : "slate"}
          action={
            !isEditing ? (
              <div className="mt-3 flex flex-col items-start gap-1.5">
                {cuisineActive && (
                  <button
                    type="button"
                    onClick={() => setHubTab("cuisine")}
                    className="text-xs font-bold text-emerald-700 hover:underline"
                  >
                    Voir le détail restauration →
                  </button>
                )}
                {canEditEffectif && (
                  <button
                    type="button"
                    onClick={openCuisineModalForOwner}
                    className="text-xs font-bold text-indigo-600 hover:underline"
                  >
                    {cuisineActive
                      ? "Modifier la commande cuisine"
                      : "Configurer une commande cuisine"}
                  </button>
                )}
              </div>
            ) : undefined
          }
        >
          {isEditing ? (
            <button
              type="button"
              onClick={openCuisineModalFromEdit}
              className={`w-full p-3 rounded-xl border-2 flex items-center justify-between transition-all text-left ${
                editedData?.piqueNiqueDetails?.active
                  ? "border-emerald-400 bg-emerald-50"
                  : "border-slate-200 bg-slate-50"
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
          ) : cuisineActive ? (
            <div>
              <p className="font-bold text-emerald-800">Commande cuisine configurée</p>
              <span className="inline-block mt-1.5 text-[10px] font-bold text-emerald-800 bg-emerald-100/80 px-2.5 py-0.5 rounded-full">
                {mealCount} repas · {cuisineDays} jour(s)
              </span>
            </div>
          ) : (
            <span className="text-slate-500">Pas de commande cuisine</span>
          )}
        </InfoTile>

        <InfoTile icon="🎯" label="Objectifs pédagogiques" tone="slate" span={2}>
          {isEditing ? (
            <TripTextarea
              value={editedData.objectifs}
              onChange={(e) => setEditedData({ ...editedData, objectifs: e.target.value })}
            />
          ) : trip.data.objectifs ? (
            <span className="whitespace-pre-wrap leading-relaxed font-normal text-slate-700">
              {trip.data.objectifs}
            </span>
          ) : (
            <span className="text-slate-400 italic font-normal">Aucun objectif renseigné.</span>
          )}
        </InfoTile>
      </div>

      {documentCount > 0 && (
        <div className="mt-4 flex items-center gap-3 rounded-xl border border-slate-100 bg-slate-50/80 px-4 py-3">
          <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-white text-sm shadow-sm">
            📎
          </span>
          <p className="text-xs text-slate-600">
            <strong className="text-slate-800">
              {documentCount} document{documentCount > 1 ? "s" : ""}
            </strong>{" "}
            dans le dossier —{" "}
            <button
              type="button"
              onClick={() => setHubTab("documents")}
              className="font-bold text-indigo-600 hover:underline"
            >
              voir l&apos;onglet Documents
            </button>
          </p>
        </div>
      )}
    </TripSection>
  );
}
