"use client";

import { useMemo } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import HomeMainTasks from "@/app/components/Dashboard/HomeMainTasks";
import HomeSignalBento from "@/app/components/Dashboard/HomeSignalBento";
import HomeScoliaHeroBar from "@/app/components/Dashboard/HomeScoliaHeroBar";
import { useData } from "@/app/contexts/data";
import { useDashboardSignals } from "@/app/hooks/useDashboardSignals";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { resolveHomeMainTasks } from "@/app/lib/home-main-tasks";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

/**
 * Accueil chat-first : barre ScolIA + tâches de rôle + signaux regroupés.
 */
export default function Home() {
  const { isLoaded, user } = useSessionUser();
  const isOrgAdmin = useIsOrgAdmin();
  const data = useData();
  const { shortcuts, notifications, loading: loadingSignals } = useDashboardSignals();

  const userRoles = useMemo(() => {
    if (!user) return [];
    return intranetRolesFromMetadata(user.publicMetadata);
  }, [user]);

  const eleveBienEtre = useMemo(() => {
    if (!user) return false;
    return isEleveBienEtreProfile(userRoles);
  }, [user, userRoles]);

  const mainTasks = useMemo(
    () =>
      resolveHomeMainTasks({
        roles: userRoles,
        accessibleModuleIds: data.accessibleModuleIds ?? null,
        orgAdmin: isOrgAdmin,
      }),
    [userRoles, data.accessibleModuleIds, isOrgAdmin],
  );

  if (!isLoaded) return null;

  return (
    <DashboardThemeRoot>
      <div className="relative min-h-[calc(100dvh-1rem)] overflow-x-hidden bg-[color:var(--dash-surface)]">
        <div className="pointer-events-none absolute inset-0" aria-hidden>
          <div className="absolute -left-24 top-0 h-[26rem] w-[26rem] rounded-full bg-[color:var(--dash-lime)]/25 blur-3xl" />
          <div className="absolute right-0 top-24 h-[20rem] w-[20rem] rounded-full bg-[color:var(--dash-soft)]/80 blur-3xl" />
        </div>

        <main className="relative mx-auto flex w-full max-w-[90rem] flex-col gap-5 px-3 py-4 sm:gap-6 sm:px-5 lg:px-7 lg:py-5">
          {!user ? (
            <div className="mx-auto mt-16 w-full max-w-sm rounded-[1.75rem] border border-black/6 bg-white/90 p-8 text-center shadow-sm">
              <h2 className="mb-2 text-2xl font-semibold text-[var(--dash-ink)]">Espace privé</h2>
              <p className="mb-8 text-sm text-neutral-500">
                Veuillez vous identifier pour accéder à ScolIA et aux services.
              </p>
              <button
                type="button"
                onClick={() => {
                  window.location.href = "/sign-in";
                }}
                className="w-full rounded-2xl bg-[var(--dash-ink)] px-8 py-4 font-semibold text-white shadow-sm transition hover:opacity-90"
              >
                Se connecter
              </button>
            </div>
          ) : eleveBienEtre ? (
            <div className="mx-auto mt-8 w-full max-w-3xl rounded-[1.75rem] border border-black/6 bg-white/80 px-6 py-10 text-center shadow-sm">
              <p className="mb-2 text-lg font-semibold text-[var(--dash-ink)]">Espace bien-être</p>
              <p className="text-sm leading-relaxed text-neutral-600">
                Ouvre la bulle en bas à droite pour parler au bot d&apos;écoute.
              </p>
            </div>
          ) : (
            <>
              <HomeScoliaHeroBar />
              <HomeMainTasks tasks={mainTasks} />
              <HomeSignalBento
                shortcuts={shortcuts}
                notifications={notifications}
                loading={loadingSignals}
              />
            </>
          )}
        </main>
      </div>
    </DashboardThemeRoot>
  );
}
