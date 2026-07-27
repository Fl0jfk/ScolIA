"use client";

import { useUser } from "@clerk/nextjs";
import { Suspense, useState, useEffect, useCallback, useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Image from "next/image";
import TravelsDirectionDashboardPanel from "@/app/components/travels/TravelsDirectionDashboard";
import {
  TravelsRemindersModal,
  type TravelsReminderRow,
} from "@/app/components/travels/TravelsRemindersModal";
import type { TravelsDirectionDashboard } from "@/app/lib/travels-direction-dashboard";
import { isTripTravelDatePast } from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { normalizeTravelImageUrl } from "@/app/lib/travels-image-url";
import { useAppContext } from "@/app/hooks/useAppContext";
import { GROUPE_SCOLAIRE_LABEL } from "@/app/lib/travels-establishments";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import {
  MODULE_TOUR_ACTION_EVENT,
  MODULE_TOUR_STEP_EVENT,
} from "@/app/lib/module-tour-actions";

function TripDashboardContent() {
  const { isLoaded, isSignedIn } = useUser();
  const { data: appCtx } = useAppContext();
  const router = useRouter();
  const searchParams = useSearchParams();
  const [showModal, setShowModal] = useState(false);
  const [trips, setTrips] = useState<TravelsTrip[]>([]);
  const [loading, setLoading] = useState(true);
  const [filterEtab, setFilterEtab] = useState("");
  const [directionDashboard, setDirectionDashboard] = useState<TravelsDirectionDashboard | null>(null);
  const [reminderCount, setReminderCount] = useState(0);
  const [reminders, setReminders] = useState<TravelsReminderRow[]>([]);
  const [showRemindersModal, setShowRemindersModal] = useState(false);
  const [tourModalBoost, setTourModalBoost] = useState(false);

  useEffect(() => {
    const onAction = (e: Event) => {
      const action = (e as CustomEvent<{ action: string }>).detail?.action;
      if (action === "travels:open-create-modal") setShowModal(true);
      if (action === "travels:close-create-modal") setShowModal(false);
    };
    const onStep = (e: Event) => {
      const target = (e as CustomEvent<{ target?: string }>).detail?.target;
      setTourModalBoost(target === "travels-type-modal");
    };
    window.addEventListener(MODULE_TOUR_ACTION_EVENT, onAction);
    window.addEventListener(MODULE_TOUR_STEP_EVENT, onStep);
    return () => {
      window.removeEventListener(MODULE_TOUR_ACTION_EVENT, onAction);
      window.removeEventListener(MODULE_TOUR_STEP_EVENT, onStep);
    };
  }, []);

  const loadTrips = useCallback(async () => {
    try {
      const res = await fetch("/api/travels/list");
      if (res.ok) {
        const data = await res.json();
        setTrips(data);
      }
    } catch (error) {
      console.error("Erreur chargement voyages:", error);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadReminders = useCallback(async () => {
    try {
      const res = await fetch("/api/travels/reminders");
      if (res.ok) {
        const data = await res.json();
        const list = Array.isArray(data.reminders) ? data.reminders : [];
        setReminders(list);
        setReminderCount(Number(data.count) || list.length);
      }
    } catch {
      setReminderCount(0);
    }
  }, []);

  const loadDirectionDashboard = useCallback(async () => {
    try {
      const res = await fetch("/api/travels/dashboard");
      if (!res.ok) return;
      const payload = await res.json();
      if (payload.isDirection && payload.dashboard) {
        setDirectionDashboard(payload.dashboard);
      }
    } catch (error) {
      console.error("Erreur dashboard voyages:", error);
    }
  }, []);

  useEffect(() => {
    if (isLoaded && isSignedIn) {
      loadTrips();
      loadDirectionDashboard();
      loadReminders();
    }
  }, [isLoaded, isSignedIn, loadTrips, loadDirectionDashboard, loadReminders]);

  useEffect(() => {
    if (searchParams.get("new") === "1") setShowModal(true);
  }, [searchParams]);

  const etabFilterOptions = useMemo(() => {
    const labels = (appCtx?.establishments || []).map((e) => e.label);
    const showGroupe = labels.length > 1;
    return { labels, showGroupe };
  }, [appCtx?.establishments]);

  const filteredTrips = useMemo(() => {
    if (!filterEtab) return trips;
    const defaultLabel = etabFilterOptions.showGroupe ? GROUPE_SCOLAIRE_LABEL : etabFilterOptions.labels[0] || "";
    return trips.filter((t) => (t.data?.etablissement || defaultLabel) === filterEtab);
  }, [trips, filterEtab, etabFilterOptions]);

  if (!isLoaded || !isSignedIn) return null;

  const formatDate = (trip: TravelsTrip, field: "created" | "travel") => {
    let val;
    if (field === 'created') { val = trip.createdAt || trip.updatedAt;
    } else { val = trip.data?.startDate || trip.data?.date;}
    if (!val) return "À préciser";
    const d = new Date(val);
    return isNaN(d.getTime()) ? val : d.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short', year: 'numeric' });
  };

  const ETAB_STYLE: Record<string, { bg: string; text: string; border: string; stripe: string }> = {
    "École":          { bg: "bg-yellow-50",  text: "text-yellow-800", border: "border-yellow-300", stripe: "bg-yellow-400" },
    "Collège":        { bg: "bg-sky-50",    text: "text-sky-800",   border: "border-sky-300",   stripe: "bg-sky-500"   },
    "Lycée":          { bg: "bg-pink-50",    text: "text-pink-800",   border: "border-pink-300",   stripe: "bg-pink-500"   },
    "Groupe Scolaire":{ bg: "bg-violet-50",  text: "text-violet-800", border: "border-violet-300", stripe: "bg-violet-500" },
  };

  const getStatusStyle = (status: string) => {
    switch (status) {
      case 'VALIDE': return 'bg-emerald-50 text-emerald-700 border-emerald-100';
      case 'REJET_MODIF': return 'bg-rose-50 text-rose-700 border-rose-100';
      case 'EN_ATTENTE_DIR_INITIAL': return 'bg-blue-50 text-blue-700 border-blue-100';
      case 'SEANCE_ANNULEE': return 'bg-slate-100 text-slate-600 border-slate-200';
      case 'ANNULE': return 'bg-red-50 text-red-700 border-red-100';
      default: return 'bg-amber-50 text-amber-700 border-amber-100';
    }
  };
  return (
    <div className="max-w-7xl mx-auto p-6 min-h-screen mt-[1vh]">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-center mb-10 gap-4">
        <div>
          <h1 className="text-4xl font-black text-slate-900 tracking-tight">Module Voyage</h1>
          <p className="text-slate-500 font-medium" data-tour="travels-reminders">
            Gestion des sorties — transport, cuisine, documents et suivi.
            {reminderCount > 0 && (
              <button
                type="button"
                onClick={() => setShowRemindersModal(true)}
                className="ml-2 inline-flex items-center px-2 py-0.5 rounded-full bg-amber-100 text-amber-800 text-xs font-bold hover:bg-amber-200 transition-colors cursor-pointer"
                title="Voir les rappels actifs"
              >
                {reminderCount} rappel{reminderCount > 1 ? "s" : ""} — cliquer pour détail
              </button>
            )}
          </p>
        </div>
        <button
          data-tour="travels-create"
          onClick={() => setShowModal(true)}
          className="bg-indigo-600 hover:bg-indigo-700 text-white px-8 py-3.5 rounded-2xl font-bold shadow-lg transition-all"
        >
          + Nouvelle demande
        </button>
      </div>
      {directionDashboard && (
        <div data-tour="travels-direction">
          <TravelsDirectionDashboardPanel data={directionDashboard} />
        </div>
      )}
      <div className="flex gap-2 flex-wrap mb-6">
        {["Tous", ...etabFilterOptions.labels, ...(etabFilterOptions.showGroupe ? [GROUPE_SCOLAIRE_LABEL] : [])].map((f) => {
          const active = (f === "Tous" && !filterEtab) || filterEtab === f;
          const s = f !== "Tous" ? ETAB_STYLE[f] ?? ETAB_STYLE[GROUPE_SCOLAIRE_LABEL] : null;
          return (
            <button
              key={f}
              onClick={() => setFilterEtab(f === "Tous" ? "" : f)}
              className={`px-4 py-2 rounded-xl text-sm font-bold border transition-all ${
                active
                  ? s ? `${s.bg} ${s.text} ${s.border} shadow-sm` : "bg-slate-900 text-white border-slate-900"
                  : "bg-white text-slate-500 border-slate-200 hover:border-slate-400"
              }`}
            >
              {f === "Tous" ? "🗂 Tous" : f === "École" ? `🏫 ${f}` : f === "Collège" ? `📚 ${f}` : f === "Lycée" ? `🎓 ${f}` : `🏛 ${f}`}
            </button>
          );
        })}
      </div>
      {loading ? (
        <div className="text-center py-20">Chargement des dossiers...</div>
      ) : filteredTrips.length > 0 ? (
        <div data-tour="travels-list" className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {filteredTrips.map((trip) => {
            const isComplex = trip.type === "COMPLEX" || Boolean((trip.data as { transport?: unknown })?.transport);
            const imageUrl = normalizeTravelImageUrl(
              (typeof trip.imageUrl === "string" && trip.imageUrl) ||
                (typeof trip.data?.imageUrl === "string" ? trip.data.imageUrl : undefined),
            );
            const defaultEtab = etabFilterOptions.showGroupe ? GROUPE_SCOLAIRE_LABEL : etabFilterOptions.labels[0] || "Établissement";
            const etabLabel = trip.data?.etablissement || defaultEtab;
            const etabStyle = ETAB_STYLE[etabLabel] ?? ETAB_STYLE[GROUPE_SCOLAIRE_LABEL];
            const isPast = isTripTravelDatePast(trip);
            return (
              <div
                key={trip.id}
                onClick={() => router.push(`/travels/${trip.id}`)}
                className={`group min-w-0 rounded-[2.5rem] overflow-hidden border transition-all duration-300 cursor-pointer transform-gpu ${
                  isPast
                    ? "bg-slate-100/90 border-slate-200 opacity-60 grayscale hover:opacity-75 hover:grayscale-[0.85]"
                    : "bg-white border-slate-200/60 shadow-sm hover:shadow-xl hover:-translate-y-1"
                }`}
              >
                <div className={`h-1.5 w-full ${etabStyle.stripe}`} />
                <div className="h-44 w-full relative bg-slate-100 overflow-hidden isolate" style={{ maskImage: 'radial-gradient(white, black)' }}>
                  {imageUrl ? (
                    <Image 
                      src={imageUrl} 
                      alt={trip.data?.title || "Sortie scolaire"}
                      className="w-full h-full object-cover transition-transform duration-700 group-hover:scale-110"
                      width={500}
                      height={300}
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center text-5xl bg-gradient-to-br from-slate-50 to-slate-100">
                      {isComplex ? '🚌' : '🍦'}
                    </div>
                  )}
                  <div className="absolute top-4 left-4 flex gap-2">
                    <span className={`text-[10px] font-black px-3 py-1.5 rounded-xl border backdrop-blur-md shadow-sm ${getStatusStyle(trip.status)}`}>
                      {trip.status === "SEANCE_ANNULEE"
                        ? "Séance annulée"
                        : trip.status?.replace('EN_ATTENTE_', '').replace('_', ' ')}
                    </span>
                  </div>
                  <div className={`absolute top-4 right-4 px-4 py-1.5 rounded-xl font-black text-sm border shadow-lg backdrop-blur-md ${etabStyle.bg} ${etabStyle.text} ${etabStyle.border}`}>
                    {etabLabel === "École" ? "🏫" : etabLabel === "Collège" ? "📚" : etabLabel === "Lycée" ? "🎓" : "🏛"} {etabLabel}
                  </div>
                </div>
                <div className="p-8">
                  <div className="flex justify-between items-center mb-4 flex-wrap gap-2">
                    <span className={`text-[11px] font-bold px-3 py-1 rounded-full border ${isComplex ? 'bg-purple-50 text-purple-700 border-purple-100' : 'bg-slate-50 text-slate-600 border-slate-100'}`}>
                      {isComplex ? 'Voyage Scolaire' : 'Sortie Locale'}
                      {trip.data?.recurrenceSeriesId && trip.data?.recurrenceTotal ? (
                        <span className="ml-2 text-indigo-600">
                          · Série {trip.data.recurrenceIndex ?? "?"}/{trip.data.recurrenceTotal}
                        </span>
                      ) : null}
                    </span>
                    <span className="text-slate-400 text-[10px] font-bold uppercase tracking-wider">
                      Dossier du {formatDate(trip, 'created')}
                    </span>
                  </div>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div className="space-y-3">
                      <h3 className="text-2xl font-black text-slate-800 group-hover:text-indigo-600 transition-colors line-clamp-1">{trip.data?.title || "Sans titre"}</h3>
                      <div className="space-y-1">
                        <p className="text-sm font-medium text-slate-500 flex items-center gap-2">
                          <span className="text-lg">📍</span> {trip.data?.destination || "Non définie"}
                        </p>
                        <p className="text-sm text-slate-500">
                          {trip.type === "COMPLEX" ? (
                            <span>
                              Du {formatDate(trip, "travel")} au{" "}
                              {trip.data?.endDate
                                ? new Date(trip.data.endDate).toLocaleDateString("fr-FR")
                                : "—"}
                            </span>
                          ) : (
                            <span>Le {formatDate(trip, "travel")}</span>
                          )}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-3 md:justify-end">
                      <div className="bg-slate-50 px-4 py-3 rounded-2xl text-center min-w-[70px] border border-slate-100">
                        <p className="text-[10px] text-slate-400 font-bold uppercase">Élèves</p>
                        <p className="text-md font-black text-slate-700">{trip.data?.nbEleves || 0}</p>
                      </div>
                      <div className="bg-slate-50 px-4 py-3 rounded-2xl text-center min-w-[80px] border border-slate-100">
                        <p className="text-[10px] text-slate-400 font-bold uppercase">Budget</p>
                        <p className="text-md font-black text-slate-700">{Math.round(trip.data?.coutTotal || 0)}€</p>
                      </div>
                    </div>
                  </div>
                  <div className="mt-8 pt-6 border-t border-slate-50 flex justify-between items-center">
                    <div className="flex items-center gap-3">
                      <div className="w-9 h-9 rounded-full bg-slate-900 flex items-center justify-center text-[10px] font-bold text-white uppercase shadow-inner">
                        {trip.ownerName?.substring(0, 2)}
                      </div>
                      <span className="text-sm font-bold text-slate-600">{trip.ownerName}</span>
                    </div>
                    <div className="flex items-center gap-2 text-indigo-600 font-bold text-sm">
                      <span className="opacity-0 group-hover:opacity-100 transition-all translate-x-2 group-hover:translate-x-0">Gérer le dossier</span>
                      <div className="w-10 h-10 rounded-full bg-slate-100 flex items-center justify-center group-hover:bg-indigo-600 group-hover:text-white transition-all shadow-sm">
                        →
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="text-center py-20 bg-white rounded-[2.5rem] border-2 border-dashed border-slate-200">
          <p className="text-slate-400 font-bold text-xl">Aucun dossier en cours.</p>
        </div>
      )}
      {showModal && (
        <div
          className={`fixed inset-0 flex items-center justify-center p-4 ${tourModalBoost ? "z-[10051]" : "z-50"}`}
        >
          <div
            className="absolute inset-0 bg-slate-900/60 backdrop-blur-sm"
            onClick={() => {
              if (!tourModalBoost) setShowModal(false);
            }}
          />
          <div
            data-tour="travels-type-modal"
            className={`relative bg-white rounded-[2.5rem] shadow-2xl max-w-xl w-full p-10 transform transition-all animate-in fade-in zoom-in duration-300 border border-white/20 ${
              tourModalBoost ? "pointer-events-none" : ""
            }`}
          >
            <div className="text-center mb-8">
              <h2 className="text-3xl font-black text-slate-900 mb-2">Nouveau Projet</h2>
              <p className="text-slate-500 font-medium">Choisissez le type de déplacement.</p>
            </div>
            <div className="grid grid-cols-1 gap-4">
              <button onClick={() => router.push("/travels/simple")} className="group p-6 bg-slate-50 border-2 border-transparent hover:border-indigo-500 hover:bg-indigo-50/50 rounded-3xl transition-all text-left flex items-center gap-6">
                <div className="w-16 h-16 bg-white rounded-2xl shadow-sm flex items-center justify-center text-3xl group-hover:scale-110 transition-transform">🍦</div>
                <div>
                  <h3 className="font-bold text-slate-800 text-lg">Sortie de proximité</h3>
                  <p className="text-sm text-slate-500 leading-snug">Sans transport spécifique (Cinéma, parc, musées...)</p>
                </div>
              </button>
              <button onClick={() => router.push("/travels/complex")} className="group p-6 bg-slate-50 border-2 border-transparent hover:border-indigo-500 hover:bg-indigo-50/50 rounded-3xl transition-all text-left flex items-center gap-6">
                <div className="w-16 h-16 bg-white rounded-2xl shadow-sm flex items-center justify-center text-3xl group-hover:scale-110 transition-transform">🚌</div>
                <div>
                  <h3 className="font-bold text-slate-800 text-lg">Voyage / Sortie Bus</h3>
                  <p className="text-sm text-slate-500 leading-snug">Transport, budget complexe ou nuitées.</p>
                </div>
              </button>
            </div>
            <button
              type="button"
              onClick={() => {
                if (!tourModalBoost) setShowModal(false);
              }}
              className="mt-8 w-full text-slate-400 hover:text-slate-600 font-bold text-sm uppercase tracking-[0.2em] transition"
            >
              Fermer la fenêtre
            </button>
          </div>
        </div>
      )}
      <TravelsRemindersModal
        open={showRemindersModal}
        reminders={reminders}
        onClose={() => setShowRemindersModal(false)}
      />
      <ReplayModuleTourButton moduleId="travels" />
    </div>
  );
}

export default function TripDashboard() {
  return (
    <Suspense fallback={<div className="p-8 text-slate-500 text-sm">Chargement des sorties…</div>}>
      <TripDashboardContent />
    </Suspense>
  );
}