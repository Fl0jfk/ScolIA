"use client";

import { CUISINE_DAYS_UI as CUISINE_DAYS, CUISINE_ROWS_UI as CUISINE_ROWS, emptyCuisineDetails } from "@/app/lib/travels-cuisine-form";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { TripButton, TripInput, TripTextarea } from "@/app/components/travels/TripDetailUI";

export type TripDetailsModalsProps = {
  trip: TravelsTrip;
  showEffectifModal: boolean;
  setShowEffectifModal: (v: boolean) => void;
  draftNbEleves: string;
  setDraftNbEleves: (v: string) => void;
  draftNbAccompagnateurs: string;
  setDraftNbAccompagnateurs: (v: string) => void;
  draftNomsAccompagnateurs: string;
  setDraftNomsAccompagnateurs: (v: string) => void;
  saveEffectifChange: () => void;
  effectifFollowUp: {
    sendTransport: boolean;
    sendCuisine: boolean;
    savedTrip: TravelsTrip;
  } | null;
  setEffectifFollowUp: (v: TripDetailsModalsProps["effectifFollowUp"]) => void;
  runEffectifFollowUp: () => void;
  showBudgetModal: boolean;
  setShowBudgetModal: (v: boolean) => void;
  draftCoutTotal: string;
  setDraftCoutTotal: (v: string) => void;
  saveBudgetChange: () => void;
  cuisineFollowUp: { mode: "initial" | "amendment"; savedTrip: TravelsTrip } | null;
  setCuisineFollowUp: (v: TripDetailsModalsProps["cuisineFollowUp"]) => void;
  runCuisineFollowUp: () => void;
  showDateModal: boolean;
  setShowDateModal: (v: boolean) => void;
  draftStartDate: string;
  setDraftStartDate: (v: string) => void;
  draftEndDate: string;
  setDraftEndDate: (v: string) => void;
  draftStartTime: string;
  setDraftStartTime: (v: string) => void;
  draftEndTime: string;
  setDraftEndTime: (v: string) => void;
  saveDateChange: () => void;
  dateFollowUp: { sendTransport: boolean; sendCuisine: boolean; savedTrip: TravelsTrip } | null;
  setDateFollowUp: (v: TripDetailsModalsProps["dateFollowUp"]) => void;
  runDateFollowUp: () => void;
  showCuisineModal: boolean;
  isEditing: boolean;
  cuisineModalStandalone: boolean;
  setShowCuisineModal: (v: boolean) => void;
  setCuisineModalStandalone: (v: boolean) => void;
  activeCuisineDetails: {
    active?: boolean;
    deliveryTime?: string;
    deliveryPlace?: string;
    daysSelection?: Record<string, boolean>;
    orders?: Record<string, Record<string, string>>;
  } | null | undefined;
  patchCuisineDetails: (fn: (prev: ReturnType<typeof emptyCuisineDetails>) => ReturnType<typeof emptyCuisineDetails>) => void;
  saveCuisineFromOwnerModal: () => void | Promise<void>;
};

export function TripDetailsModals(p: TripDetailsModalsProps) {
  const {
    trip, showEffectifModal, setShowEffectifModal, draftNbEleves, setDraftNbEleves,
    draftNbAccompagnateurs, setDraftNbAccompagnateurs, draftNomsAccompagnateurs,
    setDraftNomsAccompagnateurs, saveEffectifChange, effectifFollowUp, setEffectifFollowUp,
    runEffectifFollowUp, showBudgetModal, setShowBudgetModal, draftCoutTotal, setDraftCoutTotal,
    saveBudgetChange, cuisineFollowUp, setCuisineFollowUp, runCuisineFollowUp, showDateModal,
    setShowDateModal, draftStartDate, setDraftStartDate, draftEndDate, setDraftEndDate,
    draftStartTime, setDraftStartTime, draftEndTime, setDraftEndTime, saveDateChange,
    dateFollowUp, setDateFollowUp, runDateFollowUp, showCuisineModal, isEditing,
    cuisineModalStandalone, setShowCuisineModal, setCuisineModalStandalone, activeCuisineDetails,
    patchCuisineDetails, saveCuisineFromOwnerModal,
  } = p;
  return (
    <>
      {showEffectifModal && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[75] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-md w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Modifier effectifs &amp; accompagnateurs</h2>
            <p className="text-sm text-slate-500 mb-6">
              Créateur ou direction — mise à jour sans rouvrir tout le dossier.
            </p>
            <div className="grid grid-cols-2 gap-4 mb-4">
              <div>
                <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Élèves</label>
                <TripInput
                  type="number"
                  min={0}
                  value={draftNbEleves}
                  onChange={(e) => setDraftNbEleves(e.target.value)}
                />
              </div>
              <div>
                <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Nb accompagnateurs</label>
                <TripInput
                  type="number"
                  min={0}
                  value={draftNbAccompagnateurs}
                  onChange={(e) => setDraftNbAccompagnateurs(e.target.value)}
                />
              </div>
            </div>
            <div className="mb-6">
              <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Noms des accompagnateurs</label>
              <TripTextarea
                value={draftNomsAccompagnateurs}
                onChange={(e) => setDraftNomsAccompagnateurs(e.target.value)}
                placeholder="Ex. Mme Dupont, M. Martin…"
              />
            </div>
            <p className="text-xs text-amber-800 bg-amber-50 border border-amber-100 rounded-lg p-3 mb-6">
              Si un devis transport ou une commande cuisine a déjà été envoyé(e), vous pourrez déclencher les relances juste après l&apos;enregistrement.
            </p>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setShowEffectifModal(false)}>
                Annuler
              </TripButton>
              <TripButton variant="primary" className="flex-1" onClick={saveEffectifChange}>
                Enregistrer
              </TripButton>
            </div>
          </div>
        </div>
      )}

      {effectifFollowUp && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[80] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-lg w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Effectif enregistré</h2>
            <p className="text-sm text-slate-500 mb-5">Souhaitez-vous notifier les prestataires du changement ?</p>
            <div className="space-y-3 mb-6">
              {effectifFollowUp.sendTransport && (
                <label className="flex items-start gap-3 p-3 rounded-xl border border-amber-100 bg-amber-50 cursor-pointer">
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={effectifFollowUp.sendTransport}
                    onChange={(e) =>
                      setEffectifFollowUp({ ...effectifFollowUp, sendTransport: e.target.checked })
                    }
                  />
                  <span className="text-sm text-slate-700">
                    <strong>Demander un nouveau devis transport</strong>
                    <br />
                    <span className="text-xs text-slate-500">
                      {effectifFollowUp.savedTrip.data?.selectedBusQuote
                        ? `Uniquement ${effectifFollowUp.savedTrip.data.selectedBusQuote.providerName}.`
                        : "À tous les transporteurs référencés."}
                    </span>
                  </span>
                </label>
              )}
              {effectifFollowUp.sendCuisine && (
                <label className="flex items-start gap-3 p-3 rounded-xl border border-emerald-100 bg-emerald-50 cursor-pointer">
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={effectifFollowUp.sendCuisine}
                    onChange={(e) =>
                      setEffectifFollowUp({ ...effectifFollowUp, sendCuisine: e.target.checked })
                    }
                  />
                  <span className="text-sm text-slate-700">
                    <strong>Renvoyer la commande cuisine (annule et remplace)</strong>
                    <br />
                    <span className="text-xs text-slate-500">
                      Mail au chef avec formules de politesse et nouvelle commande en pièce jointe.
                    </span>
                  </span>
                </label>
              )}
            </div>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setEffectifFollowUp(null)}>
                Plus tard
              </TripButton>
              <TripButton
                variant="primary"
                className="flex-1"
                onClick={runEffectifFollowUp}
                disabled={
                  !effectifFollowUp.sendTransport && !effectifFollowUp.sendCuisine
                }
              >
                Envoyer les relances
              </TripButton>
            </div>
          </div>
        </div>
      )}

      {showBudgetModal && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[75] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-md w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Modifier le budget prévisionnel</h2>
            <p className="text-sm text-slate-500 mb-6">
              Montant total estimé au départ du projet (hors validation compta).
            </p>
            <div className="flex items-center gap-2 mb-6">
              <TripInput
                type="number"
                min={0}
                className="flex-1"
                value={draftCoutTotal}
                onChange={(e) => setDraftCoutTotal(e.target.value)}
              />
              <span className="text-sm font-bold text-slate-500">€</span>
            </div>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setShowBudgetModal(false)}>
                Annuler
              </TripButton>
              <TripButton variant="primary" className="flex-1" onClick={saveBudgetChange}>
                Enregistrer
              </TripButton>
            </div>
          </div>
        </div>
      )}

      {cuisineFollowUp && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[80] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-lg w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Commande cuisine enregistrée</h2>
            <p className="text-sm text-slate-500 mb-5">
              {cuisineFollowUp.mode === "initial"
                ? "Souhaitez-vous envoyer la commande au chef maintenant ?"
                : "Une commande avait déjà été envoyée — renvoyer au chef (annule et remplace) ?"}
            </p>
            <p className="text-xs text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-lg p-3 mb-6">
              {cuisineFollowUp.mode === "initial"
                ? "Un PDF sera joint au mail (chef + copies direction et organisateur)."
                : "Le mail précisera qu'il s'agit de la dernière commande en date."}
            </p>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setCuisineFollowUp(null)}>
                Plus tard
              </TripButton>
              <TripButton variant="primary" className="flex-1" onClick={runCuisineFollowUp}>
                {cuisineFollowUp.mode === "initial" ? "Envoyer au chef" : "Renvoyer (avenant)"}
              </TripButton>
            </div>
          </div>
        </div>
      )}

      {showDateModal && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[75] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-md w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Modifier dates & horaires</h2>
            <p className="text-sm text-slate-500 mb-6">Créateur ou direction — relances transport/cuisine proposées après enregistrement.</p>
            <div className="space-y-4 mb-6">
              <div className="flex gap-2 flex-wrap">
                <div className="flex-1 min-w-[8rem]">
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Date début</label>
                  <TripInput type="date" value={draftStartDate} onChange={(e) => setDraftStartDate(e.target.value)} />
                </div>
                {trip.type === "COMPLEX" && (
                  <div className="flex-1 min-w-[8rem]">
                    <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Date fin</label>
                    <TripInput type="date" value={draftEndDate} onChange={(e) => setDraftEndDate(e.target.value)} />
                  </div>
                )}
              </div>
              <div className="flex gap-2">
                <div className="flex-1">
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Départ</label>
                  <TripInput type="time" value={draftStartTime} onChange={(e) => setDraftStartTime(e.target.value)} />
                </div>
                <div className="flex-1">
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Retour</label>
                  <TripInput type="time" value={draftEndTime} onChange={(e) => setDraftEndTime(e.target.value)} />
                </div>
              </div>
            </div>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setShowDateModal(false)}>Annuler</TripButton>
              <TripButton variant="primary" className="flex-1" onClick={saveDateChange}>Enregistrer</TripButton>
            </div>
          </div>
        </div>
      )}

      {dateFollowUp && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[80] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-lg w-full shadow-2xl">
            <h2 className="text-xl font-bold text-slate-900 mb-1">Dates enregistrées</h2>
            <p className="text-sm text-slate-500 mb-5">Notifier les prestataires du changement de planning ?</p>
            <div className="space-y-3 mb-6">
              {dateFollowUp.sendTransport && (
                <label className="flex items-center gap-3 p-3 rounded-xl border border-amber-100 bg-amber-50">
                  <input type="checkbox" checked={dateFollowUp.sendTransport} onChange={(e) => setDateFollowUp({ ...dateFollowUp, sendTransport: e.target.checked })} />
                  <span className="text-sm"><strong>Relancer le transporteur</strong> (avenant dates)</span>
                </label>
              )}
              {dateFollowUp.sendCuisine && (
                <label className="flex items-center gap-3 p-3 rounded-xl border border-emerald-100 bg-emerald-50">
                  <input type="checkbox" checked={dateFollowUp.sendCuisine} onChange={(e) => setDateFollowUp({ ...dateFollowUp, sendCuisine: e.target.checked })} />
                  <span className="text-sm"><strong>Renvoyer commande cuisine</strong></span>
                </label>
              )}
            </div>
            <div className="flex gap-3">
              <TripButton variant="secondary" className="flex-1" onClick={() => setDateFollowUp(null)}>Plus tard</TripButton>
              <TripButton variant="primary" className="flex-1" onClick={runDateFollowUp} disabled={!dateFollowUp.sendTransport && !dateFollowUp.sendCuisine}>Envoyer</TripButton>
            </div>
          </div>
        </div>
      )}

      {showCuisineModal && (isEditing || cuisineModalStandalone) && (
        <div className="fixed inset-0 bg-slate-900/70 backdrop-blur-md flex items-center justify-center z-[70] p-4">
          <div className="bg-white rounded-2xl p-6 sm:p-8 max-w-5xl w-full shadow-2xl overflow-y-auto max-h-[90vh]">
            <div className="flex justify-between items-start mb-6 border-b border-slate-100 pb-4">
              <div>
                <h2 className="text-xl font-bold text-slate-900">Bon de commande cuisine</h2>
                <p className="text-slate-500 text-sm mt-0.5">
                  {cuisineModalStandalone
                    ? "Ajouter ou modifier la commande — envoi au chef proposé après enregistrement."
                    : "Configuration de la commande restauration"}
                </p>
              </div>
              <TripButton
                variant="ghost"
                size="sm"
                onClick={() => {
                  setShowCuisineModal(false);
                  setCuisineModalStandalone(false);
                }}
              >
                ✕
              </TripButton>
            </div>
            <div className="space-y-5">
              {cuisineModalStandalone && (
                <label className="flex items-center gap-3 p-4 rounded-xl border-2 border-slate-200 bg-slate-50 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={!!activeCuisineDetails?.active}
                    onChange={(e) =>
                      patchCuisineDetails((prev) => ({
                        ...prev,
                        active: e.target.checked,
                      }))
                    }
                  />
                  <span className="text-sm font-bold text-slate-800">Commander une restauration (pique-nique / self)</span>
                </label>
              )}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4 bg-slate-50 p-4 rounded-2xl">
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Heure récupération / livraison</label>
                  <input
                    type="time"
                    className="w-full p-2 border rounded-lg"
                    value={activeCuisineDetails?.deliveryTime || ""}
                    disabled={cuisineModalStandalone && !activeCuisineDetails?.active}
                    onChange={(e) =>
                      patchCuisineDetails((prev) => ({
                        ...prev,
                        active: true,
                        deliveryTime: e.target.value,
                      }))
                    }
                  />
                </div>
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-1">Lieu de récupération</label>
                  <select
                    className="w-full p-2 border rounded-lg"
                    value={activeCuisineDetails?.deliveryPlace || "Self"}
                    disabled={cuisineModalStandalone && !activeCuisineDetails?.active}
                    onChange={(e) =>
                      patchCuisineDetails((prev) => ({
                        ...prev,
                        active: true,
                        deliveryPlace: e.target.value,
                      }))
                    }
                  >
                    <option value="Self">Au self</option>
                    <option value="Bosco">Église Bosco</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold uppercase text-slate-500 mb-2 text-center">Jours concernés</label>
                  <div className="flex gap-1.5 justify-center">
                    {CUISINE_DAYS.map(({ key: dayKey, label }) => {
                      const isSelected = !!activeCuisineDetails?.daysSelection?.[dayKey];
                      return (
                        <button
                          key={dayKey}
                          type="button"
                          disabled={cuisineModalStandalone && !activeCuisineDetails?.active}
                          onClick={() =>
                            patchCuisineDetails((prev) => ({
                              ...prev,
                              active: true,
                              daysSelection: {
                                ...(prev.daysSelection || emptyCuisineDetails().daysSelection),
                                [dayKey]: !isSelected,
                              },
                            }))
                          }
                          className={`w-9 h-9 rounded-lg text-[11px] font-black transition-all ${isSelected ? 'bg-indigo-600 text-white shadow-md' : 'bg-white border-2 text-slate-400 hover:border-indigo-300'}`}
                        >
                          {label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
              <div className="overflow-x-auto rounded-xl border border-slate-200">
                <table className="w-full text-xs border-collapse min-w-[620px]">
                  <thead>
                    <tr className="bg-indigo-600 text-white">
                      <th className="text-left p-2.5 font-semibold w-52">Désignation</th>
                      {CUISINE_DAYS.map(({ key: dayKey, label }) => (
                        <th key={dayKey} className={`p-2.5 text-center font-semibold transition-opacity ${activeCuisineDetails?.daysSelection?.[dayKey] ? "opacity-100" : "opacity-30"}`}>{label}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {CUISINE_ROWS.map(({ key: rowKey, label, type }, rowIdx) => (
                      <tr key={rowKey} className={rowIdx % 2 === 0 ? 'bg-white' : 'bg-slate-50'}>
                        <td className={`p-2 font-medium text-slate-700 whitespace-nowrap ${rowKey === 'picnicNoPork' || rowKey === 'picnicVeg' ? 'pl-5 text-slate-500 italic' : ''}`}>{label}</td>
                        {CUISINE_DAYS.map(({ key: dayKey }) => {
                          const isActive = !!activeCuisineDetails?.daysSelection?.[dayKey];
                          const val = activeCuisineDetails?.orders?.[dayKey]?.[rowKey] ?? "";
                          return (
                            <td key={dayKey} className="p-1">
                              <input
                                type={type}
                                disabled={!isActive || (cuisineModalStandalone && !activeCuisineDetails?.active)}
                                value={val}
                                onChange={(e) =>
                                  patchCuisineDetails((prev) => {
                                    const day = dayKey as keyof ReturnType<typeof emptyCuisineDetails>["orders"];
                                    return {
                                      ...prev,
                                      active: true,
                                      orders: {
                                        ...(prev.orders || emptyCuisineDetails().orders),
                                        [day]: {
                                          ...(prev.orders?.[day] ?? {}),
                                          [rowKey]: e.target.value,
                                        },
                                      },
                                    };
                                  })
                                }
                                className={`w-full p-1.5 border rounded text-center transition-all ${isActive ? 'bg-white hover:border-indigo-300 focus:border-indigo-500 outline-none' : 'bg-slate-100 text-slate-300 cursor-not-allowed border-transparent'}`}
                              />
                            </td>
                          );
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="text-[10px] text-slate-500 italic bg-amber-50 border border-amber-100 p-2.5 rounded-lg">⚠️ Rappel : fournir la liste des élèves/adultes 15 jours avant, et l’affiner 24h avant.</p>
            </div>
            <div className="flex gap-3 mt-8 pt-4 border-t border-slate-100">
              {isEditing && !cuisineModalStandalone && (
                <TripButton
                  variant="danger"
                  className="flex-1"
                  onClick={() => {
                    patchCuisineDetails((prev) => ({ ...prev, active: false }));
                    setShowCuisineModal(false);
                  }}
                >
                  Annuler la commande
                </TripButton>
              )}
              <TripButton
                variant="secondary"
                className="flex-1"
                onClick={() => {
                  setShowCuisineModal(false);
                  setCuisineModalStandalone(false);
                }}
              >
                Fermer
              </TripButton>
              <TripButton
                variant="primary"
                className="flex-[2]"
                onClick={() => {
                  if (cuisineModalStandalone) {
                    void saveCuisineFromOwnerModal();
                  } else {
                    setShowCuisineModal(false);
                  }
                }}
              >
                {cuisineModalStandalone ? "Enregistrer la commande" : "Enregistrer le bon"}
              </TripButton>
            </div>
          </div>
        </div>
      )}

    </>
  );
}
