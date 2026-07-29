"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import { motion } from "framer-motion";
import { useUser } from "@clerk/nextjs";
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
  DASHBOARD_FOOTER_ADMIN_MODULE_IDS,
  DASHBOARD_PILLARS,
  pillarHasVisibleModules,
} from "@/app/lib/dashboard-pillars";
import { toDashboardQuickLinks } from "@/app/lib/dashboard-quick-links";
import type { DashboardShortcut, DashboardTodayNewsItem } from "@/app/lib/dashboard-signals";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

function fingerprint(shortcuts: DashboardShortcut[], news: DashboardTodayNewsItem[]) {
  return [
    ...shortcuts.filter((s) => s.rich).map((s) => `${s.id}:${s.badge}:${s.detail}`),
    ...news.map((n) => n.id),
  ].join("|");
}

export default function Home() {
  const { isLoaded, user } = useUser();
  const isOrgAdmin = useIsOrgAdmin();
  const data = useData();

  const [shortcuts, setShortcuts] = useState<DashboardShortcut[]>([]);
  const [todayNews, setTodayNews] = useState<DashboardTodayNewsItem[]>([]);
  const [hasCurrentWeek, setHasCurrentWeek] = useState(false);
  const [signalsLoading, setSignalsLoading] = useState(true);
  const [pulseKey, setPulseKey] = useState("");
  const prevFp = useRef("");

  const firstName =
    user?.firstName ||
    user?.fullName?.split(/\s+/)[0] ||
    user?.username ||
    null;

  const uniqueCategories = useMemo(() => {
    if (!isLoaded || !user || !data?.categories) return [];
    const roles = intranetRolesFromMetadata(user.publicMetadata);
    const filtered = data.categories.filter((category) => {
      if (category.orgAdminOnly) return isOrgAdmin;
      if (hasGlobalAdminRole(roles)) return true;
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
    const rawRoles = user.publicMetadata?.role;
    const roles = Array.isArray(rawRoles) ? rawRoles : typeof rawRoles === "string" ? [rawRoles] : [];
    return toDashboardQuickLinks(
      data.externalQuickLinks.filter((l) => (l.allowedRoles ?? []).some((r) => roles.includes(r))),
    );
  }, [isLoaded, user, data]);

  const eleveBienEtre = useMemo(() => {
    if (!user) return false;
    return isEleveBienEtreProfile(intranetRolesFromMetadata(user.publicMetadata));
  }, [user]);

  const applySignals = useCallback(
    (json: {
      shortcuts?: DashboardShortcut[];
      todayNews?: DashboardTodayNewsItem[];
      hasCurrentWeek?: boolean;
    }) => {
      const nextShortcuts = Array.isArray(json.shortcuts) ? json.shortcuts : [];
      const nextNews = Array.isArray(json.todayNews) ? json.todayNews : [];
      const fp = fingerprint(nextShortcuts, nextNews);
      if (prevFp.current && prevFp.current !== fp) {
        const changed = nextShortcuts
          .filter((s) => s.rich)
          .map((s) => s.id)
          .join(",");
        setPulseKey(`${Date.now()}:${changed}`);
      }
      prevFp.current = fp;
      setShortcuts(nextShortcuts);
      setTodayNews(nextNews);
      setHasCurrentWeek(Boolean(json.hasCurrentWeek));
    },
    [],
  );

  const loadSignals = useCallback(async () => {
    if (!user) {
      setSignalsLoading(false);
      return;
    }
    try {
      const res = await fetch("/api/dashboard/signals", { cache: "no-store" });
      if (!res.ok) return;
      applySignals(await res.json());
    } catch {
      /* ignore */
    } finally {
      setSignalsLoading(false);
    }
  }, [user, applySignals]);

  useEffect(() => {
    if (!isLoaded) return;
    void loadSignals();
  }, [isLoaded, loadSignals]);

  // Live refresh — signaux dynamiques (absences, demandes, etc.)
  useEffect(() => {
    if (!isLoaded || !user) return;
    const id = window.setInterval(() => {
      void loadSignals();
    }, 35000);
    const onFocus = () => void loadSignals();
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(id);
      window.removeEventListener("focus", onFocus);
    };
  }, [isLoaded, user, loadSignals]);

  const adminCats = useMemo(
    () =>
      dashboardCategories.filter((c) =>
        (DASHBOARD_FOOTER_ADMIN_MODULE_IDS as readonly string[]).includes(c.moduleId),
      ),
    [dashboardCategories],
  );

  const hasPillars = DASHBOARD_PILLARS.some((p) =>
    pillarHasVisibleModules(p, dashboardCategories),
  );

  if (!isLoaded) return null;

  return (
    <DashboardThemeRoot>
      <div className="relative overflow-hidden">
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

        <main className="relative mx-auto flex min-h-[calc(100dvh-4.5rem)] w-full max-w-[1600px] flex-col px-4 sm:px-6 lg:h-[calc(100dvh-4.5rem)] lg:max-h-[calc(100dvh-4.5rem)] lg:overflow-hidden lg:px-8">
          <div className="flex min-h-0 flex-1 flex-col gap-3 py-3 lg:gap-3.5 lg:overflow-hidden lg:py-4">
            <header className="grid shrink-0 grid-cols-1 gap-3 lg:grid-cols-[minmax(0,auto)_minmax(0,1fr)_minmax(0,auto)] lg:items-end lg:gap-x-8">
              <motion.div
                className="min-w-0"
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.45 }}
              >
                <p className="text-[10px] font-semibold uppercase tracking-[0.24em] text-[var(--dash-mid)]">
                  Tableau de bord
                </p>
                <h1 className="mt-1 text-[1.75rem] font-semibold tracking-tight text-[var(--dash-ink)] md:text-[2.15rem]">
                  {firstName ? (
                    <>
                      Bonjour{" "}
                      <span className="bg-gradient-to-r from-[var(--dash-primary)] via-[var(--dash-mid)] to-[var(--dash-bright)] bg-clip-text text-transparent">
                        {firstName}
                      </span>
                    </>
                  ) : (
                    "Bienvenue"
                  )}
                </h1>
              </motion.div>
              <div className="justify-self-start lg:pl-2">
                <DashboardWeather />
              </div>
              <div className="justify-self-start lg:justify-self-end">
                <ExternalQuickLinksBar links={quickLinks} />
              </div>
            </header>

            <div className="shrink-0">
              <DashboardTodayNews
                items={todayNews}
                hasCurrentWeek={hasCurrentWeek}
                loading={signalsLoading}
                onWeekSheetUpdated={loadSignals}
              />
            </div>

            <div className="flex min-h-0 flex-1 flex-col gap-3 lg:overflow-hidden">
              {hasPillars ? (
                <DashboardPillars
                  categories={dashboardCategories}
                  shortcuts={shortcuts}
                  pulseKey={pulseKey}
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

              {adminCats.length > 0 ? (
                <footer className="flex shrink-0 flex-wrap items-center justify-center gap-2 pt-1">
                  {adminCats.map((c) => (
                    <Link
                      key={c.moduleId}
                      href={c.link}
                      className="rounded-full border border-white/70 bg-white/55 px-4 py-2 text-xs font-semibold text-[var(--dash-ink)] backdrop-blur transition hover:bg-white/80"
                    >
                      {c.name}
                    </Link>
                  ))}
                </footer>
              ) : null}
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
                className="w-full rounded-2xl bg-gradient-to-r from-[var(--dash-primary)] to-[var(--dash-dark)] px-8 py-4 font-semibold text-white shadow-lg transition hover:brightness-110"
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
