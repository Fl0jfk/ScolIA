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

/** Tuile légère — demi-largeur desktop, zéro décoration. */
function Tile({
  icon,
  label,
  children,
  action,
}: {
  icon: string;
  label: string;
  children: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="rounded-xl border border-slate-100 bg-white px-4 py-3">
      <p className="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wide text-slate-400">
        <span aria-hidden className="opacity-80">
          {icon}
        </span>
        {label}
      </p>
      <div className="mt-1.5 text-sm font-medium text-slate-800 leading-snug min-w-0">
        {children}
      </div>
      {action ? <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1">{action}</div> : null}
    </div>
  );
}

function LinkBtn({
  onClick,
  disabled,
  children,
  tone = "indigo",
}: {
  onClick: () => void;
  disabled?: boolean;
  children: ReactNode;
  tone?: "indigo" | "amber" | "emerald";
}) {
  const tones = {
    indigo: "text-indigo-600",
    amber: "text-amber-700",
    emerald: "text-emerald-700",
  };
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`text-xs font-semibold hover:underline disabled:opacity-50 ${tones[tone]}`}
    >
      {children}
    </button>
  );
}

function EmptyValue() {
  return <span className="text-slate-400 font-normal">—</span>;
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
  } = p;

  const cuisineActive = Boolean(trip.data.piqueNiqueDetails?.active);
  const mealCount = cuisineActive
    ? getTotalMeals(trip.data.piqueNiqueDetails ?? emptyCuisineDetails())
    : 0;
  const cuisineDays = cuisineActive
    ? Object.values(trip.data.piqueNiqueDetails?.daysSelection || {}).filter(Boolean).length
    : 0;

  const accompagnateursLabel = trip.data.nomsAccompagnateurs
    ? String(trip.data.nomsAccompagnateurs)
    : Number(trip.data.nbAccompagnateurs || 0) > 0
      ? `${trip.data.nbAccompagnateurs} (noms à préciser)`
      : "Aucun";

  return (
    <TripSection title="Détails du dossier" icon="📋">
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <Tile icon="📍" label="Destination">
          {trip.data.destination ? (
            <span className="line-clamp-2" title={trip.data.destination}>
              {trip.data.destination}
            </span>
          ) : (
            <EmptyValue />
          )}
        </Tile>

        <Tile icon="🏫" label="Classes">
          {isEditing ? (
            <TripClassesMultiSelect
              value={String(editedData.classes || "")}
              options={classOptions}
              onChange={(classes) => setEditedData({ ...editedData, classes })}
            />
          ) : trip.data.classes ? (
            <span className="line-clamp-2" title={trip.data.classes}>
              {trip.data.classes}
            </span>
          ) : (
            <EmptyValue />
          )}
        </Tile>

        <Tile
          icon="👥"
          label="Effectifs"
          action={
            !isEditing ? (
              <>
                {canEditEffectif && (
                  <LinkBtn onClick={openEffectifModal}>Modifier</LinkBtn>
                )}
                {withBusLogistics && effectifChanged && (
                  <LinkBtn
                    tone="amber"
                    disabled={loadingAction === "amendment-quote"}
                    onClick={() => requestAmendedBusQuote()}
                  >
                    Devis transport rectifié
                  </LinkBtn>
                )}
                {cuisineOrderSent && cuisineActive && cuisineChanged && (
                  <LinkBtn
                    tone="emerald"
                    disabled={loadingAction === "cuisine-amendment"}
                    onClick={() => sendCuisineAmendment()}
                  >
                    Renvoyer cuisine
                  </LinkBtn>
                )}
              </>
            ) : undefined
          }
        >
          {isEditing ? (
            <div className="flex gap-2">
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
            <span>
              {trip.data.nbEleves ?? 0} él. · {trip.data.nbAccompagnateurs || 0} acc.
            </span>
          )}
        </Tile>

        <Tile
          icon="🧑‍🏫"
          label="Accompagnateurs"
          action={
            !isEditing && canEditEffectif ? (
              <LinkBtn onClick={openEffectifModal}>Modifier</LinkBtn>
            ) : undefined
          }
        >
          {isEditing ? (
            <div className="space-y-2">
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
            <span className="line-clamp-2 font-normal text-slate-700" title={accompagnateursLabel}>
              {accompagnateursLabel}
            </span>
          )}
        </Tile>

        <Tile
          icon="📅"
          label="Dates"
          action={
            !isEditing ? (
              <>
                {canEditDates && <LinkBtn onClick={openDateModal}>Modifier</LinkBtn>}
                {datesChanged && (
                  <span className="text-[10px] font-semibold text-amber-700">
                    Modifiées depuis l’envoi transport
                  </span>
                )}
              </>
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
            <span>{dateLabel || <EmptyValue />}</span>
          )}
        </Tile>

        <Tile icon="🕐" label="Horaires">
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
              {trip.data.startTime || "—"} → {trip.data.endTime || "—"}
            </span>
          )}
        </Tile>

        <Tile
          icon="💶"
          label="Budget"
          action={
            !isEditing ? (
              <>
                {canEditEffectif && (
                  <LinkBtn onClick={openBudgetModal}>Modifier</LinkBtn>
                )}
                {canAccessComptaTab && (
                  <LinkBtn onClick={() => setHubTab("compta")}>Compta</LinkBtn>
                )}
              </>
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
              <span className="text-xs text-slate-500">€</span>
            </div>
          ) : (
            <span>
              {Math.round(Number(trip.data.coutTotal) || 0)} €
              {trip.data.finalTotalCost ? (
                <span className="text-emerald-700 font-semibold">
                  {" "}
                  · validé {trip.data.finalTotalCost} €
                </span>
              ) : (
                <span className="text-slate-400 font-normal"> prév.</span>
              )}
            </span>
          )}
        </Tile>

        <Tile
          icon="🥪"
          label="Restauration"
          action={
            !isEditing ? (
              <>
                {cuisineActive && (
                  <LinkBtn tone="emerald" onClick={() => setHubTab("cuisine")}>
                    Détail
                  </LinkBtn>
                )}
                {canEditEffectif && (
                  <LinkBtn onClick={openCuisineModalForOwner}>
                    {cuisineActive ? "Modifier" : "Configurer"}
                  </LinkBtn>
                )}
              </>
            ) : undefined
          }
        >
          {isEditing ? (
            <button
              type="button"
              onClick={openCuisineModalFromEdit}
              className="text-left text-sm font-semibold text-indigo-600 hover:underline"
            >
              {editedData?.piqueNiqueDetails?.active
                ? `${getTotalMeals(editedData.piqueNiqueDetails)} repas — modifier`
                : "Configurer la commande"}
            </button>
          ) : cuisineActive ? (
            <span>
              {mealCount} repas · {cuisineDays} j.
            </span>
          ) : (
            <span className="text-slate-400 font-normal">Aucune</span>
          )}
        </Tile>

        <Tile icon="🎯" label="Objectifs">
          {isEditing ? (
            <TripTextarea
              value={editedData.objectifs}
              onChange={(e) => setEditedData({ ...editedData, objectifs: e.target.value })}
            />
          ) : trip.data.objectifs ? (
            <span
              className="line-clamp-3 font-normal text-slate-700 whitespace-pre-wrap"
              title={trip.data.objectifs}
            >
              {trip.data.objectifs}
            </span>
          ) : (
            <EmptyValue />
          )}
        </Tile>
      </div>
    </TripSection>
  );
}
