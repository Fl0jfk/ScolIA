"use client";

import { useEffect, useState, type ReactNode } from "react";
import { usePathname } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import EleveDossierModalProvider from "@/app/components/shell/EleveDossierModalProvider";
import IntranetSidebar from "@/app/components/shell/IntranetSidebar";

export default function IntranetShell({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const { isSignedIn, isLoaded } = useSessionUser();
  const [mobileOpen, setMobileOpen] = useState(false);

  const isOnboarding =
    pathname === "/onboarding" || pathname.startsWith("/onboarding/");

  const shellActive = Boolean(isLoaded && isSignedIn && !isOnboarding);

  useEffect(() => {
    setMobileOpen(false);
  }, [pathname]);

  useEffect(() => {
    const root = document.documentElement;
    if (shellActive) root.dataset.intranetShell = "1";
    else delete root.dataset.intranetShell;
    return () => {
      delete root.dataset.intranetShell;
    };
  }, [shellActive]);

  if (!shellActive) {
    return <>{children}</>;
  }

  return (
    <DashboardThemeRoot>
      <EleveDossierModalProvider>
        <div className="min-h-screen lg:pl-[17.5rem]">
          <IntranetSidebar
            mobileOpen={mobileOpen}
            onCloseMobile={() => setMobileOpen(false)}
          />

          <div className="sticky top-0 z-30 flex h-14 items-center gap-3 border-b border-black/6 bg-[color:var(--dash-surface)]/95 px-4 backdrop-blur-sm lg:hidden print:!hidden">
            <button
              type="button"
              className="inline-flex h-10 w-10 flex-col items-center justify-center gap-[5px] rounded-xl bg-white shadow-sm ring-1 ring-black/5"
              onClick={() => setMobileOpen(true)}
              aria-label="Ouvrir le menu"
            >
              <span className="block h-[1.5px] w-5 rounded-full bg-[var(--dash-ink)]" />
              <span className="block h-[1.5px] w-3.5 rounded-full bg-[var(--dash-ink)]" />
              <span className="block h-[1.5px] w-5 rounded-full bg-[var(--dash-ink)]" />
            </button>
            <p className="text-sm font-bold text-[var(--dash-ink)]">Menu</p>
          </div>

          <div className="min-w-0">{children}</div>
        </div>
      </EleveDossierModalProvider>
    </DashboardThemeRoot>
  );
}
