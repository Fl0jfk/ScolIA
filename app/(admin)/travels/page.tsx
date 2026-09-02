"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { Suspense, useState, useEffect, useCallback, useMemo } from "react";
import dynamic from "next/dynamic";
import { useRouter, useSearchParams } from "next/navigation";
import Image from "next/image";
import TravelsDirectionDashboardPanel from "@/app/components/travels/TravelsDirectionDashboard";
import {
  TravelsRemindersModal,
  type TravelsReminderRow,
} from "@/app/components/travels/TravelsRemindersModal";
import type { TravelsDirectionDashboard } from "@/app/lib/travels-direction-dashboard";
import { isTripTravelDatePast } from "@/app/lib/travels-trip-helpers";
import { TRAVELS_STATUS_LABELS, type TravelsTrip } from "@/app/lib/travels-types";
import { normalizeTravelImageUrl } from "@/app/lib/travels-image-url";
import { useAppContext } from "@/app/hooks/useAppContext";
import { GROUPE_SCOLAIRE_LABEL } from "@/app/lib/travels-establishments";
import { visualForEstablishmentLabel } from "@/app/lib/establishment-visual";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleEmptyState from "@/app/components/module-chrome/ModuleEmptyState";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import {
  MODULE_TOUR_ACTION_EVENT,
  MODULE_TOUR_STEP_EVENT,
} from "@/app/lib/module-tour-actions";
import { canEnterTravelsDetail } from "@/app/lib/accueil-access";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";

type TravelsMainTab = "dossiers" | "settings";

const TravelsTransportSettingsPanel = dynamic(
  () => import("@/app/components/travels/TravelsTransportSettingsPanel"),
  { ssr: false, loading: () => <ModuleTabFallback /> },
);

function TripDashboardContent() {
  const { isLoaded, isSignedIn, user } = useSessionUser();
  const { data: appCtx } = useAppContext();
  const isOrgAdmin = useIsOrgAdmin();
  const router = useRouter();
  const searchParams = useSearchParams();
  const roles = useMemo(() => rolesFromUserLike(user), [user]);
  const canOpenTrip = useMemo(
    () =>
      canEnterTravelsDetail({
        roles,
        orgAdmin: isOrgAdmin,
        platformAdmin: Boolean(appCtx?.session?.isGlobalAdmin),
      }),
    [roles, isOrgAdmin, appCtx?.session?.isGlobalAdmin],
  );
  const [showModal, setShowModal] = useState(false);
  const [trips, setTrips] = useState<TravelsTrip[]>([]);
  const [loading, setLoading] = useState(true);
  const [filterEtab, setFilterEtab] = useState("");
  const [directionDashboard, setDirectionDashboard] = useState<TravelsDirectionDashboard | null>(null);
  const [reminderCount, setReminderCount] = useState(0);
  const [reminders, setReminders] = useState<TravelsReminderRow[]>([]);
  const [showRemindersModal, setShowRemindersModal] = useState(false);
  const [tourModalBoost, setTourModalBoost] = useState(false);
  const [mainTab, setMainTab] = useState<TravelsMainTab>("dossiers");

  useEffect(() => {
    if (searchParams.get("tab") === "settings" && isOrgAdmin) {
      setMainTab("settings");
    }
  }, [searchParams, isOrgAdmin]);

  useEffect(() => {
    const onAction = (e: Event) => {
      const action = (e as CustomEvent<{ action: string }>).detail?.action;
      if (action === "travels:open-create-modal" && canOpenTrip) setShowModal(true);
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
  }, [canOpenTrip]);

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
    if (searchParams.get("new") === "1" && canOpenTrip) setShowModal(true);
  }, [searchParams, canOpenTrip]);

  const etabFilterOptions = useMemo(() => {
    const establishments = (appCtx?.establishments || []).filter((e) => e.active !== false);
    const labels = establishments.map((e) => e.label);
    const showGroupe = labels.length > 1;
    return { labels, showGroupe, establishments };
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

  const etabVisual = (label: string) =>
    visualForEstablishmentLabel(label, etabFilterOptions.establishments, GROUPE_SCOLAIRE_LABEL);

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
    <ModulePageShell maxWidthClass="max-w-[1500px]" tourModuleId="travels">
      <ModulePageHeader
        title="Module Voyage"
        description={
          <p data-tour="travels-reminders">
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
        }
        actions={
          mainTab === "dossiers" && canOpenTrip ? (
            <ModuleButton data-tour="travels-create" onClick={() => setShowModal(true)}>
              + Nouvelle demande
            </ModuleButton>
          ) : undefined
        }
      />

      <ModuleTabNav
        className="mb-6"
        tabs={[
          { id: "dossiers", label: "Dossiers" },
          { id: "settings", label: "Paramétrage", hidden: !isOrgAdmin },
        ]}
        active={mainTab}
        onChange={setMainTab}
      />

      {mainTab === "settings" && isOrgAdmin ? (
        <TravelsTransportSettingsPanel />
      ) : (
        <>
      {directionDashboard && (
        <div data-tour="travels-direction">
          <TravelsDirectionDashboardPanel data={directionDashboard} />
        </div>
      )}
      <div className="flex gap-2 flex-wrap mb-6">
        {["Tous", ...etabFilterOptions.labels, ...(etabFilterOptions.showGroupe ? [GROUPE_SCOLAIRE_LABEL] : [])].map((f) => {
          const active = (f === "Tous" && !filterEtab) || filterEtab === f;
          const vis = f !== "Tous" ? etabVisual(f) : null;
          return (
            <button
              key={f}
              onClick={() => setFilterEtab(f === "Tous" ? "" : f)}
              className={`px-4 py-2 rounded-xl text-sm font-bold border transition-all cursor-pointer ${
                active
                  ? vis
                    ? "shadow-sm"
                    : "bg-slate-900 text-white border-slate-900"
                  : "bg-white text-slate-500 border-slate-200 hover:border-slate-400"
              }`}
              style={
                active && vis
                  ? {
                      backgroundColor: vis.badgeBg,
                      color: vis.textColor,
                      borderColor: vis.borderColor,
                    }
                  : undefined
              }
            >
              {f === "Tous" ? "Tous" : f}
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
            const vis = etabVisual(etabLabel);
            const isPast = isTripTravelDatePast(trip);
            return (
              <div
                key={trip.id}
                onClick={canOpenTrip ? () => router.push(`/travels/${trip.id}`) : undefined}
                className={`group relative min-w-0 rounded-[2.5rem] overflow-hidden border transition-all duration-300 transform-gpu ${
                  canOpenTrip ? "cursor-pointer" : "cursor-default"
                } ${
                  isPast
                    ? "bg-slate-100/90 border-slate-200 opacity-60 grayscale hover:opacity-75 hover:grayscale-[0.85]"
                    : canOpenTrip
                      ? "shadow-sm hover:shadow-xl hover:-translate-y-1"
                      : "shadow-sm"
                }`}
                title={
                  canOpenTrip
                    ? undefined
                    : "Consultation liste uniquement — ouverture du dossier réservée à d’autres rôles"
                }
                style={
                  isPast
                    ? undefined
                    : { backgroundColor: vis.washBg, borderColor: vis.borderColor }
                }
              >
                {isPast ? null : (
                  <div
                    className="pointer-events-none absolute -right-10 -bottom-12 h-44 w-44 rounded-full blur-3xl transition duration-700 group-hover:scale-110"
                    style={{ backgroundColor: vis.orbBg }}
                    aria-hidden
                  />
                )}
                <div className="relative h-44 w-full bg-slate-100 overflow-hidden isolate" style={{ maskImage: 'radial-gradient(white, black)' }}>
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
                      {TRAVELS_STATUS_LABELS[trip.status] ||
                        (trip.status === "SEANCE_ANNULEE"
                          ? "Séance annulée"
                          : trip.status?.replace("EN_ATTENTE_", "").replace("_", " "))}
                    </span>
                  </div>
                  <div
                    className="absolute top-4 right-4 inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-[10px] font-bold uppercase tracking-wide border shadow-sm"
                    style={{
                      backgroundColor: vis.badgeBg,
                      color: vis.textColor,
                      borderColor: vis.borderColor,
                    }}
                  >
                    <span
                      className="h-1.5 w-1.5 rounded-full shrink-0"
                      style={{ backgroundColor: vis.hex }}
                      aria-hidden
                    />
                    {etabLabel}
                  </div>
                </div>
                <div className="relative p-8">
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
        <ModuleEmptyState className="py-20 rounded-[2.5rem]">
          <p className="font-bold text-xl">Aucun dossier en cours.</p>
        </ModuleEmptyState>
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
        </>
      )}
    </ModulePageShell>
  );
}

export default function TripDashboard() {
  return (
    <Suspense
      fallback={
        <ModulePageShell maxWidthClass="max-w-[1500px]">
          <p className="text-slate-500 text-sm">Chargement des sorties…</p>
        </ModulePageShell>
      }
    >
      <TripDashboardContent />
    </Suspense>
  );
}