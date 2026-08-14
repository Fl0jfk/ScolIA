"use client";

import { CUISINE_DAYS_UI as CUISINE_DAYS, CUISINE_ROWS_UI as CUISINE_ROWS, getTotalMeals } from "@/app/lib/travels-cuisine-form";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { TripAlert, TripButton, TripSection } from "@/app/components/travels/TripDetailUI";

type TripCuisineHubPanelProps = {
  trip: TravelsTrip;
  cuisineOrderSent: boolean;
  cuisineOrderSentAt: string | null | undefined;
  isOwner: boolean;
  canSign: boolean;
  loadingAction: string | null;
  sendCuisineAmendment: () => void;
  cuisineChanged: boolean;
  dateLabel: string;
};

export function TripCuisineHubPanel({
  trip, cuisineOrderSent, cuisineOrderSentAt, isOwner, canSign, loadingAction,
  sendCuisineAmendment, cuisineChanged, dateLabel,
}: TripCuisineHubPanelProps) {
  return (
        <TripSection
          title="Commande restauration"
          subtitle="Bon de commande envoyé au chef ou en préparation"
          icon="🍽️"
          accent="emerald"
          action={
            cuisineOrderSent && (isOwner || canSign) ? (
              <TripButton
                variant="warning"
                size="sm"
                onClick={() => sendCuisineAmendment()}
                disabled={loadingAction === "cuisine-amendment"}
              >
                {loadingAction === "cuisine-amendment" ? "Envoi…" : "Annule et remplace"}
              </TripButton>
            ) : undefined
          }
        >
          {(() => {
            const details = trip.data.piqueNiqueDetails as {
              deliveryTime?: string;
              deliveryPlace?: string;
              daysSelection?: Record<string, boolean>;
              orders?: Record<string, Record<string, string>>;
            };
            const selectedDayKeys = CUISINE_DAYS.filter((d) => details?.daysSelection?.[d.key]);
            return (
              <div className="space-y-5">
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
                  <div className="rounded-xl bg-emerald-50 border border-emerald-100 p-4">
                    <p className="text-[10px] font-bold uppercase text-emerald-700">Livraison</p>
                    <p className="font-bold text-slate-900 mt-1">
                      {details?.deliveryPlace || "—"} à {details?.deliveryTime || "—"}
                    </p>
                  </div>
                  <div className="rounded-xl bg-slate-50 border border-slate-100 p-4">
                    <p className="text-[10px] font-bold uppercase text-slate-500">Repas commandés</p>
                    <p className="font-bold text-slate-900 mt-1">{getTotalMeals(trip.data.piqueNiqueDetails)} au total</p>
                  </div>
                  <div className="rounded-xl bg-slate-50 border border-slate-100 p-4">
                    <p className="text-[10px] font-bold uppercase text-slate-500">Envoi au chef</p>
                    <p className="font-bold text-slate-900 mt-1">
                      {cuisineOrderSent
                        ? `Envoyé le ${new Date(cuisineOrderSentAt!).toLocaleDateString("fr-FR")}`
                        : trip.status === "VALIDE"
                          ? "Statut validé — envoi cuisine non tracé dans le dossier"
                          : "Pas encore envoyé (validation finale)"}
                    </p>
                    {(trip.data.cuisineAmendments?.length || 0) > 0 && (
                      <p className="text-[10px] text-amber-700 mt-1">{trip.data.cuisineAmendments!.length} rectification(s)</p>
                    )}
                  </div>
                </div>
                {cuisineChanged && (
                  <TripAlert tone="warning" title="Effectif modifié depuis la dernière commande">
                    <button
                      type="button"
                      onClick={() => sendCuisineAmendment()}
                      className="text-xs font-bold text-emerald-800 underline mt-1"
                    >
                      Renvoyer la commande au chef
                    </button>
                  </TripAlert>
                )}
                <div className="overflow-x-auto rounded-xl border border-slate-200">
                  <table className="w-full text-xs border-collapse min-w-[520px]">
                    <thead>
                      <tr className="bg-emerald-600 text-white">
                        <th className="text-left p-2.5 font-semibold">Désignation</th>
                        {selectedDayKeys.map((d) => (
                          <th key={d.key} className="p-2.5 text-center font-semibold">{d.label}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {CUISINE_ROWS.map((row, rowIdx) => (
                        <tr key={row.key} className={rowIdx % 2 === 0 ? "bg-white" : "bg-slate-50"}>
                          <td className="p-2 font-medium text-slate-700">{row.label}</td>
                          {selectedDayKeys.map((d) => (
                            <td key={d.key} className="p-2 text-center text-slate-600">
                              {details?.orders?.[d.key]?.[row.key] || "—"}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                <p className="text-[10px] text-slate-500 italic">
                  Effectif dossier : {trip.data.nbEleves} élèves, {trip.data.nbAccompagnateurs || 0} accompagnateurs — {dateLabel}
                </p>
              </div>
            );
          })()}
        </TripSection>

  );
}
