"use client";

import { useMemo } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import ScoliaHub from "@/app/components/scolia/ScoliaHub";
import { isEleveBienEtreProfile } from "@/app/lib/bien-etre-profile";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

/**
 * Accueil chat-first : ScolIA est le hero.
 * Les signaux métier vivent dans les hubs piliers / modules.
 */
export default function Home() {
  const { isLoaded, user } = useSessionUser();

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
      <div className="relative min-h-[calc(100dvh-1rem)] overflow-x-hidden bg-[color:var(--dash-surface)]">
        <div className="pointer-events-none absolute inset-0" aria-hidden>
          <div className="absolute -left-20 top-8 h-[22rem] w-[22rem] rounded-full bg-[color:var(--dash-lime)]/20 blur-3xl" />
          <div className="absolute right-0 top-0 h-[18rem] w-[18rem] rounded-full bg-[color:var(--dash-soft)]/70 blur-3xl" />
        </div>

        <main className="relative mx-auto flex min-h-[calc(100dvh-1rem)] w-full max-w-[1600px] flex-col px-3 sm:px-5 lg:px-6">
          <div className="flex flex-1 flex-col py-3 lg:py-3.5">
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
              <ScoliaHub />
            )}
          </div>
        </main>
      </div>
    </DashboardThemeRoot>
  );
}
