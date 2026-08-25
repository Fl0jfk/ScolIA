"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import { motion } from "framer-motion";
import DashboardGlobalNotifications from "@/app/components/Dashboard/DashboardGlobalNotifications";
import { useSessionUser } from "@/app/hooks/useAppUser";
import DashboardPillars from "@/app/components/Dashboard/DashboardPillars";
import DashboardTodayNews from "@/app/components/Dashboard/DashboardTodayNews";
import DashboardWeather from "@/app/components/Dashboard/DashboardWeather";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import { ExternalQuickLinksBar } from "@/app/components/Dashboard/ExternalQuickLinks";
import { useData } from "@/app/contexts/data";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { hasRole } from "@/app/lib/absences-types";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import {
  DASHBOARD_PILLARS,
  pillarHasVisibleModules,
} from "@/app/lib/dashboard-pillars";
import { toDashboardQuickLinks } from "@/app/lib/dashboard-quick-links";
import type {
  DashboardNotification,
  DashboardShortcut,
  DashboardTodayNewsItem,
} from "@/app/lib/dashboard-signals";
import { useDashboardSignals } from "@/app/hooks/useDashboardSignals";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

function fingerprint(
  shortcuts: DashboardShortcut[],
  news: DashboardTodayNewsItem[],
  notifications: DashboardNotification[],
) {
  return [
    ...shortcuts.filter((s) => s.rich).map((s) => `${s.id}:${s.badge}:${s.detail}`),
    ...news.map((n) => n.id),
    ...notifications.map((n) => `${n.id}:${n.count}`),
  ].join("|");
}

export default function Home() {
  const { isLoaded, user } = useSessionUser();
  const isOrgAdmin = useIsOrgAdmin();
  const data = useData();

  const [pulseKey, setPulseKey] = useState("");
  const prevFp = useRef("");

  const onSignalsFetched = useCallback(
    (payload: {
      shortcuts: DashboardShortcut[];
      todayNews: DashboardTodayNewsItem[];
      notifications: DashboardNotification[];
    }) => {
      const fp = fingerprint(payload.shortcuts, payload.todayNews, payload.notifications);
      if (prevFp.current && prevFp.current !== fp) {
        const changed = payload.shortcuts
          .filter((s) => s.rich)
          .map((s) => s.id)
          .join(",");
        setPulseKey(`${Date.now()}:${changed}`);
      }
      prevFp.current = fp;
    },
    [],
  );

  const {
    shortcuts,
    todayNews,
    notifications,
    hasCurrentWeek,
    anneeScolaireLabel,
    loading: signalsLoading,
    refresh: loadSignals,
  } = useDashboardSignals({ onFetched: onSignalsFetched });

  const firstName =
    user?.firstName ||
    user?.fullName?.split(/\s+/)[0] ||
    user?.username ||
    null;

  const uniqueCategories = useMemo(() => {
    if (!isLoaded || !user || !data?.categories) return [];
    const roles = intranetRolesFromMetadata(user.publicMetadata);
    const filtered = data.categories.filter((category) => {
      // Admin établissement : accès complet au catalogue (dont Paramètres).
      if (isOrgAdmin || hasGlobalAdminRole(roles)) return true;
      if (category.orgAdminOnly) return false;
      return (category.allowedRoles ?? []).some((r) => hasRole(roles, r));
    });
    return Array.from(new Map(filtered.map((cat) => [cat.moduleId, cat])).values());
  }, [isLoaded, user, data, isOrgAdmin]);

  const dashboardCategories = useMemo(
    () => uniqueCategories.filter((c) => c.moduleId !== "dashboard-week-sheet"),
    [uniqueCategories],
  );

  const quickLinks = useMemo(() => {
    if (!isLoaded || !user || !data?.externalQuickLinks) return [];
    const roles = intranetRolesFromMetadata(user.publicMetadata);
    return toDashboardQuickLinks(
      data.externalQuickLinks.filter((l) => (l.allowedRoles ?? []).some((r) => roles.includes(r))),
    );
  }, [isLoaded, user, data]);

  const userRoles = useMemo(() => {
    if (!user) return [];
    return intranetRolesFromMetadata(user.publicMetadata);
  }, [user]);

  const eleveBienEtre = useMemo(() => {
    if (!user) return false;
    return isEleveBienEtreProfile(userRoles);
  }, [user, userRoles]);

  const hasPillars = DASHBOARD_PILLARS.some((p) =>
    pillarHasVisibleModules(p, dashboardCategories, userRoles, { orgAdmin: isOrgAdmin }),
  );

  if (!isLoaded) return null;

  return (
    <DashboardThemeRoot>
      <div className="relative overflow-x-hidden">
        {/* Atmosphere */}
        <div className="pointer-events-none absolute inset-0" aria-hidden>
          <div className="absolute -left-24 top-0 h-[28rem] w-[28rem] rounded-full bg-[color:var(--dash-soft)]/80 blur-3xl" />
          <div className="absolute right-0 top-24 h-[22rem] w-[22rem] rounded-full bg-[color:var(--dash-bright)]/20 blur-3xl" />
          <div className="absolute bottom-0 left-1/3 h-[18rem] w-[18rem] rounded-full bg-[color:var(--dash-mid)]/15 blur-3xl" />
          <motion.div
            className="absolute left-1/2 top-10 h-40 w-40 -translate-x-1/2 rounded-full bg-white/40 blur-2xl"
            animate={{ opacity: [0.35, 0.55, 0.35], scale: [1, 1.08, 1] }}
            transition={{ duration: 8, repeat: Infinity, ease: "easeInOut" }}
          />
        </div>

        <main className="relative mx-auto flex min-h-[calc(100dvh-4.5rem)] w-full max-w-[1600px] flex-col px-4 sm:px-6 lg:px-8">
          <div className="flex flex-1 flex-col gap-3 py-3 lg:gap-3.5 lg:py-4">
            <header className="hidden shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-4 md:grid">
              <motion.div
                className="min-w-0 justify-self-start"
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.45 }}
              >
                <p className="text-[10px] font-semibold uppercase tracking-[0.24em] text-[var(--dash-mid)]">
                  Tableau de bord
                  {anneeScolaireLabel ? (
                    <span className="ml-2 font-bold normal-case tracking-normal text-[var(--dash-primary)]">
                      · {anneeScolaireLabel}
                    </span>
                  ) : null}
                </p>
                <h1 className="mt-1 text-[1.75rem] font-semibold tracking-tight text-[var(--dash-ink)] md:text-[2rem] lg:text-[2.15rem]">
                  {firstName ? (
                    <>
                      Bonjour{" "}
                      <span className="relative inline-flex items-start">
                        <span className="bg-gradient-to-r from-[var(--dash-primary)] via-[var(--dash-mid)] to-[var(--dash-bright)] bg-clip-text text-transparent">
                          {firstName}
                        </span>
                        <DashboardGlobalNotifications items={notifications} />
                      </span>
                    </>
                  ) : (
                    "Bienvenue"
                  )}
                </h1>
              </motion.div>

              <div className="justify-self-center">
                <DashboardTodayNews
                  items={todayNews}
                  hasCurrentWeek={hasCurrentWeek}
                  loading={signalsLoading}
                  onWeekSheetUpdated={loadSignals}
                />
              </div>

              <div className="justify-self-end">
                <DashboardWeather />
              </div>
            </header>

            {/* Mobile */}
            <div className="flex shrink-0 flex-col gap-2 md:hidden">
              <motion.div
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.45 }}
              >
                <p className="text-[10px] font-semibold uppercase tracking-[0.24em] text-[var(--dash-mid)]">
                  Tableau de bord
                  {anneeScolaireLabel ? (
                    <span className="ml-2 font-bold normal-case tracking-normal text-[var(--dash-primary)]">
                      · {anneeScolaireLabel}
                    </span>
                  ) : null}
                </p>
                <h1 className="mt-1 text-[1.75rem] font-semibold tracking-tight text-[var(--dash-ink)]">
                  {firstName ? (
                    <>
                      Bonjour{" "}
                      <span className="relative inline-flex items-start">
                        <span className="bg-gradient-to-r from-[var(--dash-primary)] via-[var(--dash-mid)] to-[var(--dash-bright)] bg-clip-text text-transparent">
                          {firstName}
                        </span>
                        <DashboardGlobalNotifications items={notifications} />
                      </span>
                    </>
                  ) : (
                    "Bienvenue"
                  )}
                </h1>
              </motion.div>
              <DashboardWeather />
              <DashboardTodayNews
                wide={false}
                items={todayNews}
                hasCurrentWeek={hasCurrentWeek}
                loading={signalsLoading}
                onWeekSheetUpdated={loadSignals}
              />
              <ExternalQuickLinksBar
                links={quickLinks}
                manageHref={isOrgAdmin ? "/parametres?tab=dashboard-links" : null}
              />
            </div>

            <div className="flex flex-1 flex-col gap-3">
              {hasPillars ? (
                <DashboardPillars
                  categories={dashboardCategories}
                  shortcuts={shortcuts}
                  notifications={notifications}
                  pulseKey={pulseKey}
                  roles={userRoles}
                  orgAdmin={isOrgAdmin}
                />
              ) : (
                user && (
                  <div className="mx-auto mt-4 w-full max-w-3xl rounded-[1.5rem] border border-white/60 bg-white/60 px-6 py-10 text-center shadow-sm backdrop-blur-xl">
                    {eleveBienEtre ? (
                      <>
                        <p className="mb-2 text-lg font-semibold text-violet-900">Espace bien-être</p>
                        <p className="text-sm leading-relaxed text-stone-600">
                          Ouvre la bulle en bas à droite pour parler au bot d&apos;écoute.
                        </p>
                      </>
                    ) : (
                      <p className="text-sm font-medium text-stone-500">
                        Aucun contenu disponible pour votre profil.
                      </p>
                    )}
                  </div>
                )
              )}
            </div>
          </div>
        </main>

        {!user ? (
          <div className="absolute inset-0 z-50 flex items-center justify-center backdrop-blur-md">
            <div className="mx-4 flex w-full max-w-sm flex-col items-center rounded-[1.75rem] border border-white/70 bg-white/80 p-8 shadow-xl backdrop-blur-xl">
              <div className="mb-6 flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-[var(--dash-primary)] to-[var(--dash-dark)] text-3xl text-white shadow-lg">
                🔒
              </div>
              <h2 className="mb-2 text-2xl font-semibold text-[var(--dash-ink)]">Espace privé</h2>
              <p className="mb-8 text-center text-sm leading-relaxed text-stone-500">
                Veuillez vous identifier pour accéder à vos services.
              </p>
              <button
                type="button"
                onClick={() => {
                  window.location.href = "/sign-in";
                }}
                className="w-full cursor-pointer rounded-2xl bg-gradient-to-r from-[var(--dash-primary)] to-[var(--dash-dark)] px-8 py-4 font-semibold text-white shadow-lg transition hover:brightness-110"
              >
                Se connecter
              </button>
            </div>
          </div>
        ) : null}
      </div>
    </DashboardThemeRoot>
  );
}
