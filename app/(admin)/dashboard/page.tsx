"use client";

import { useMemo } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import { ExternalQuickLinksBar } from "@/app/components/Dashboard/ExternalQuickLinks";
import ScoliaHub from "@/app/components/scolia/ScoliaHub";
import { useData } from "@/app/contexts/data";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { toDashboardQuickLinks } from "@/app/lib/dashboard-quick-links";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

/**
 * Accueil chat-first : ScolIA est le hero.
 * Les signaux métier vivent désormais dans les hubs piliers / modules.
 */
export default function Home() {
  const { isLoaded, user } = useSessionUser();
  const isOrgAdmin = useIsOrgAdmin();
  const data = useData();

  const quickLinks = useMemo(() => {
    if (!isLoaded || !user || !data?.externalQuickLinks) return [];
    const roles = intranetRolesFromMetadata(user.publicMetadata);
    const filtered = isOrgAdmin
      ? data.externalQuickLinks
      : data.externalQuickLinks.filter((l) =>
          (l.allowedRoles ?? []).some((r) => roles.includes(r)),
        );
    return toDashboardQuickLinks(filtered);
  }, [isLoaded, user, data, isOrgAdmin]);

  const userRoles = useMemo(() => {
    if (!user) return [];
    return intranetRolesFromMetadata(user.publicMetadata);
  }, [user]);

  const eleveBienEtre = useMemo(() => {
    if (!user) return false;
    return isEleveBienEtreProfile(userRoles);
  }, [user, userRoles]);

  if (!isLoaded) return null;

  return (
    <DashboardThemeRoot>
      <div className="relative overflow-x-hidden">
        <div className="pointer-events-none absolute inset-0" aria-hidden>
          <div className="absolute -left-24 top-0 h-[28rem] w-[28rem] rounded-full bg-[color:var(--dash-soft)]/80 blur-3xl" />
          <div className="absolute right-0 top-24 h-[22rem] w-[22rem] rounded-full bg-[color:var(--dash-bright)]/20 blur-3xl" />
          <div className="absolute bottom-0 left-1/3 h-[18rem] w-[18rem] rounded-full bg-[color:var(--dash-mid)]/15 blur-3xl" />
        </div>

        <main className="relative mx-auto flex min-h-[calc(100dvh-1rem)] w-full max-w-[1600px] flex-col px-4 sm:px-6 lg:px-8">
          <div className="flex flex-1 flex-col gap-3 py-3 lg:py-4">
            <div className="md:hidden">
              <ExternalQuickLinksBar
                links={quickLinks}
                manageHref={isOrgAdmin ? "/parametres?tab=dashboard-links" : null}
              />
            </div>

            {!user ? (
              <div className="mx-auto mt-16 w-full max-w-sm rounded-[1.75rem] border border-white/70 bg-white/80 p-8 text-center shadow-xl backdrop-blur-xl">
                <h2 className="mb-2 text-2xl font-semibold text-[var(--dash-ink)]">Espace privé</h2>
                <p className="mb-8 text-sm text-stone-500">
                  Veuillez vous identifier pour accéder à ScolIA et aux services.
                </p>
                <button
                  type="button"
                  onClick={() => {
                    window.location.href = "/sign-in";
                  }}
                  className="w-full rounded-2xl bg-gradient-to-r from-[var(--dash-primary)] to-[var(--dash-dark)] px-8 py-4 font-semibold text-white shadow-lg"
                >
                  Se connecter
                </button>
              </div>
            ) : eleveBienEtre ? (
              <div className="mx-auto mt-8 w-full max-w-3xl rounded-[1.5rem] border border-white/60 bg-white/60 px-6 py-10 text-center shadow-sm backdrop-blur-xl">
                <p className="mb-2 text-lg font-semibold text-violet-900">Espace bien-être</p>
                <p className="text-sm leading-relaxed text-stone-600">
                  Ouvre la bulle en bas à droite pour parler au bot d&apos;écoute.
                </p>
              </div>
            ) : (
              <ScoliaHub />
            )}
          </div>
        </main>
      </div>
    </DashboardThemeRoot>
  );
}
