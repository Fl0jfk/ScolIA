"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { Suspense, useState, useEffect, useCallback, useMemo } from "react";
import dynamic from "next/dynamic";
import { useRouter, useSearchParams } from "next/navigation";
import Image from "next/image";
import TravelsDirectionDashboardPanel from "@/app/components/travels/TravelsDirectionDashboard";
import TravelsTripBentoCard from "@/app/components/travels/TravelsTripBentoCard";
import {
  TravelsRemindersModal,
  type TravelsReminderRow,
} from "@/app/components/travels/TravelsRemindersModal";
import type { TravelsDirectionDashboard } from "@/app/lib/travels-direction-dashboard";
import {
  filterTripsForModuleList,
  travelsTripMatchesSearch,
} from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { normalizeTravelImageUrl } from "@/app/lib/travels-image-url";
import { useAppContext } from "@/app/hooks/useAppContext";
import { GROUPE_SCOLAIRE_LABEL } from "@/app/lib/travels-establishments";
import {
  vibrantVisualForEstablishmentLabel,
} from "@/app/lib/establishment-visual";
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
import { TravelsAssistanceButton } from "@/app/components/travels/TravelsAssistanceButton";
import { useTravelsAssistanceCard } from "@/app/hooks/useTravelsAssistanceCard";

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
  const roles = useMemo(() => {
    const fromContext = appCtx?.session?.intranetRoles;
    if (Array.isArray(fromContext) && fromContext.length > 0) return fromContext;
    return rolesFromUserLike(user);
  }, [appCtx?.session?.intranetRoles, user]);
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
  const [searchQuery, setSearchQuery] = useState("");
  const [directionDashboard, setDirectionDashboard] = useState<TravelsDirectionDashboard | null>(null);
  const [reminderCount, setReminderCount] = useState(0);
  const [reminders, setReminders] = useState<TravelsReminderRow[]>([]);
  const [showRemindersModal, setShowRemindersModal] = useState(false);
  const [tourModalBoost, setTourModalBoost] = useState(false);
  const [mainTab, setMainTab] = useState<TravelsMainTab>("dossiers");
  const { status: assistanceCardStatus } = useTravelsAssistanceCard(isLoaded && isSignedIn);

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
    const defaultLabel = etabFilterOptions.showGroupe ? GROUPE_SCOLAIRE_LABEL : etabFilterOptions.labels[0] || "";
    return filterTripsForModuleList(trips, {
      matchesSearch: (t) => travelsTripMatchesSearch(t, searchQuery),
    }).filter((t) => {
      if (!filterEtab) return true;
      return (t.data?.etablissement || defaultLabel) === filterEtab;
    });
  }, [trips, filterEtab, searchQuery, etabFilterOptions]);

  const unreadSummary = useMemo(() => {
    let tripCount = 0;
    let messageCount = 0;
    for (const t of trips) {
      const n = t.unreadInternalCount ?? 0;
      if (n <= 0) continue;
      tripCount += 1;
      messageCount += n;
    }
    return { tripCount, messageCount };
  }, [trips]);

  const unreadSummary = useMemo(() => {
    let tripCount = 0;
    let messageCount = 0;
    for (const t of trips) {
      const n = t.unreadInternalCount ?? 0;
      if (n <= 0) continue;
      tripCount += 1;
      messageCount += n;
    }
    return { tripCount, messageCount };
  }, [trips]);

  if (!isLoaded || !isSignedIn) return null;

  const formatDate = (trip: TravelsTrip, field: "created" | "travel") => {
    let val;
    if (field === 'created') { val = trip.createdAt || trip.updatedAt;
    } else { val = trip.data?.startDate || trip.data?.date;}
    if (!val) return "À préciser";
    const d = new Date(val);
    return isNaN(d.getTime()) ? val : d.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short', year: 'numeric' });
  };

  const etabCardVisual = (label: string) =>
    vibrantVisualForEstablishmentLabel(label, etabFilterOptions.establishments, GROUPE_SCOLAIRE_LABEL);
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
            {unreadSummary.messageCount > 0 && (
              <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded-full bg-red-100 text-red-800 text-xs font-bold">
                {unreadSummary.messageCount} message
                {unreadSummary.messageCount > 1 ? "s" : ""} non lu
                {unreadSummary.messageCount > 1 ? "s" : ""}
                {unreadSummary.tripCount > 1 ? ` · ${unreadSummary.tripCount} séjours` : ""}
              </span>
            )}
          </p>
        }
        actions={
          mainTab === "dossiers" ? (
            <div className="flex flex-wrap items-center gap-2">
              <TravelsAssistanceButton status={assistanceCardStatus} size="md" />
              {canOpenTrip ? (
                <ModuleButton data-tour="travels-create" onClick={() => setShowModal(true)}>
                  + Nouvelle demande
                </ModuleButton>
              ) : null}
            </div>
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
      <div className="mb-6 space-y-3">
        <label className="block">
          <span className="sr-only">Rechercher un séjour</span>
          <div className="relative">
            <span
              className="pointer-events-none absolute inset-y-0 left-4 flex items-center text-slate-400"
              aria-hidden
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="7" />
                <path d="M20 20l-3.5-3.5" />
              </svg>
            </span>
            <input
              type="search"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Rechercher un séjour, un lieu ou un professeur…"
              autoComplete="off"
              data-tour="travels-search"
              className="w-full rounded-2xl border border-slate-200 bg-white py-3.5 pl-12 pr-4 text-sm font-medium text-slate-800 shadow-sm outline-none placeholder:text-slate-400 focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100"
            />
          </div>
          <p className="mt-1.5 text-xs text-slate-500">
            Recherche dans le titre, la destination et le nom du professeur (ou accompagnateur).
          </p>
        </label>
        <div className="flex gap-2 flex-wrap">
          {["Tous", ...etabFilterOptions.labels, ...(etabFilterOptions.showGroupe ? [GROUPE_SCOLAIRE_LABEL] : [])].map((f) => {
            const active = (f === "Tous" && !filterEtab) || filterEtab === f;
            const vis = f !== "Tous" ? etabCardVisual(f) : null;
            return (
              <button
                key={f}
                type="button"
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
      </div>
      {loading ? (
        <div className="text-center py-20">Chargement des dossiers...</div>
      ) : filteredTrips.length > 0 ? (
        <div
          data-tour="travels-list"
          className="grid grid-cols-1 gap-6 lg:grid-cols-2"
        >
          {filteredTrips.map((trip, tripIndex) => {
            const defaultEtab = etabFilterOptions.showGroupe
              ? GROUPE_SCOLAIRE_LABEL
              : etabFilterOptions.labels[0] || "Établissement";
            const etabLabel = trip.data?.etablissement || defaultEtab;
            return (
              <TravelsTripBentoCard
                key={trip.id}
                trip={trip}
                etabLabel={etabLabel}
                vis={etabCardVisual(etabLabel)}
                canOpenTrip={canOpenTrip}
                index={tripIndex}
                unreadInternalCount={trip.unreadInternalCount ?? 0}
                formatDate={formatDate}
                onOpen={() => router.push(`/travels/${trip.id}`)}
              />
            );
          })}
        </div>
      ) : (
        <ModuleEmptyState className="py-20 rounded-[2.5rem]">
          <p className="font-bold text-xl">
            {searchQuery.trim() || filterEtab
              ? "Aucun séjour ne correspond à votre recherche."
              : "Aucun dossier en cours."}
          </p>
          {(searchQuery.trim() || filterEtab) && (
            <button
              type="button"
              onClick={() => {
                setSearchQuery("");
                setFilterEtab("");
              }}
              className="mt-4 text-sm font-bold text-indigo-600 hover:text-indigo-800"
            >
              Réinitialiser les filtres
            </button>
          )}
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