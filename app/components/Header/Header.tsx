"use client";

import { useState, useEffect, useMemo, useRef, type CSSProperties } from "react";
import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";
import { useSignOutWithPortalReset } from "@/app/hooks/useSignOutWithPortalReset";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useAdminBootstrap } from "@/app/contexts/admin-bootstrap";
import { useData } from "@/app/contexts/data";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { ExternalQuickLinksBar } from "@/app/components/Dashboard/ExternalQuickLinks";
import { toDashboardQuickLinks } from "@/app/lib/dashboard-quick-links";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { dashboardBrandCssVars, parseDashboardAccent } from "@/app/lib/dashboard-brand-presets";
import { SCOLA_HEADER_ACCENT } from "@/app/lib/marketing-theme";
import GlassLayer from "@/app/components/GlassLayer";
import AccountSecurityDialog from "@/app/components/Header/AccountSecurityDialog";
import Logo from "../../../public/Logo header.png";

const MOBILE_MODULE_LINKS = [
  { href: "/documents", label: "Cloud personnel", icon: "📁" },
  { href: "/requests?nouvelle=1", label: "Faire une demande", icon: "📋" },
  { href: "/prof-room?new=1", label: "Faire une réservation de salle", icon: "🏫" },
  { href: "/rh?tab=absences&view=se-declarer#nouvelle-absence", label: "Déclarer une absence", icon: "📅" },
] as const;

function UserPopover({
  onClose,
  onOpenSecurity,
}: {
  onClose: () => void;
  onOpenSecurity: () => void;
}) {
  const { user } = useSessionUser();
  const signOutWithPortalReset = useSignOutWithPortalReset();
  return (
    <div className="absolute top-12 right-0 z-50 w-64 animate-in overflow-hidden rounded-2xl border border-slate-100 bg-white shadow-2xl">
      <div className="flex items-center gap-3 border-b border-slate-100 px-4 py-4">
        {user?.imageUrl ? (
          <img
            src={user.imageUrl}
            alt={user.fullName ?? ""}
            className="h-10 w-10 flex-shrink-0 rounded-full object-cover ring-2 ring-slate-100"
          />
        ) : (
          <div className="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-blue-100 text-sm font-black text-blue-600">
            {(user?.firstName?.[0] ?? "?").toUpperCase()}
          </div>
        )}
        <div className="min-w-0">
          <p className="truncate text-sm font-bold text-slate-900">
            {user?.fullName ?? user?.username ?? "—"}
          </p>
          <p className="truncate text-xs text-slate-400">
            {user?.primaryEmailAddress?.emailAddress ?? user?.username ?? ""}
          </p>
        </div>
      </div>
      <div className="flex flex-col gap-0.5 p-2">
        <Link
          href="/dashboard"
          onClick={onClose}
          className="flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium text-slate-700 transition-colors hover:bg-slate-50"
        >
          <span className="text-base">🏠</span> Tableau de bord
        </Link>
        <button
          type="button"
          onClick={() => {
            onOpenSecurity();
            onClose();
          }}
          className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm font-medium text-slate-700 transition-colors hover:bg-slate-50"
        >
          <span className="text-base">⚙️</span> E-mail & mot de passe
        </button>
        <div className="my-1 h-px bg-slate-100" />
        <button
          onClick={() => {
            signOutWithPortalReset("/");
            onClose();
          }}
          className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm font-medium text-red-500 transition-colors hover:bg-red-50"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2} stroke="currentColor" className="h-4 w-4">
            <path strokeLinecap="round" strokeLinejoin="round" d="M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15M12 9l-3 3m0 0 3 3m-3-3h12.75" />
          </svg>
          Se déconnecter
        </button>
      </div>
    </div>
  );
}

export default function Header() {
  const [mobileOpen, setMobileOpen] = useState(false);
  const [popoverOpen, setPopoverOpen] = useState(false);
  const [securityOpen, setSecurityOpen] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);
  const pathname = usePathname();
  const { isSignedIn, user, isLoaded } = useSessionUser();
  const signOutWithPortalReset = useSignOutWithPortalReset();
  const { appContext, sitePublic: siteIdentity, loading: bootstrapLoading } = useAdminBootstrap();
  const data = useData();
  const isOrgAdmin = useIsOrgAdmin();

  useEffect(() => {
    setMobileOpen(false);
    setPopoverOpen(false);
  }, [pathname]);

  useEffect(() => {
    document.body.style.overflow = mobileOpen ? "hidden" : "";
    return () => {
      document.body.style.overflow = "";
    };
  }, [mobileOpen]);

  useEffect(() => {
    const handle = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setPopoverOpen(false);
      }
    };
    document.addEventListener("mousedown", handle);
    return () => document.removeEventListener("mousedown", handle);
  }, []);

  const isDashboard = pathname === "/dashboard" || pathname.startsWith("/dashboard/");
  const isOnboardingFlow =
    pathname === "/onboarding" || pathname.startsWith("/onboarding/");

  const dashVars = isDashboard
    ? dashboardBrandCssVars(parseDashboardAccent(appContext?.identity?.dashboardAccent))
    : null;
  const homeHref = isSignedIn ? "/dashboard" : "/";
  const logoAlt = siteIdentity?.shortName || siteIdentity?.name || "Établissement";
  const customLogoUrl = siteIdentity?.headerLogoUrl?.trim() || "";

  const headerQuickLinks = useMemo(() => {
    if (!isLoaded || !isSignedIn || !user || !data?.externalQuickLinks) return [];
    const roles = rolesFromUserLike(user);
    return toDashboardQuickLinks(
      data.externalQuickLinks.filter((l) => (l.allowedRoles ?? []).some((r) => roles.includes(r))),
    );
  }, [isLoaded, isSignedIn, user, data]);

  const headerStyle =
    isDashboard && dashVars
      ? ({ borderBottomColor: dashVars["--dash-border"], ...dashVars } as CSSProperties)
      : undefined;

  if (isOnboardingFlow) return null;

  return (
    <>
      <header
        className={`relative sticky top-0 z-50 border-b print:!hidden ${
          isDashboard ? "dashboard-themed" : "border-emerald-200/50"
        }`}
        style={headerStyle}
      >
        <GlassLayer className="bg-white/90 backdrop-blur-md" />
        <div
          className={
            isDashboard && dashVars
              ? "pointer-events-none absolute inset-x-0 bottom-0 h-px"
              : SCOLA_HEADER_ACCENT
          }
          style={
            isDashboard && dashVars
              ? {
                  background: `linear-gradient(to right, transparent, ${dashVars["--dash-bright"]}80, transparent)`,
                }
              : undefined
          }
          aria-hidden
        />
        <div className="relative z-[1] mx-auto flex h-14 max-w-[1600px] items-center gap-3 px-4 sm:px-6 lg:px-8">
          <Link href={homeHref} className="group flex shrink-0 items-center transition hover:opacity-90">
            <div className="flex h-10 min-w-[56px] shrink-0 items-center justify-center">
              {!bootstrapLoading &&
                (customLogoUrl ? (
                  <Image
                    src={customLogoUrl}
                    alt={logoAlt}
                    width={180}
                    height={48}
                    unoptimized
                    className="h-auto max-h-12 w-auto max-w-[180px] object-contain [image-rendering:auto]"
                  />
                ) : (
                  <Image
                    src={Logo}
                    alt={logoAlt}
                    width={56}
                    height={56}
                    className="drop-shadow-[0_3px_10px_rgba(47,107,74,0.22)]"
                  />
                ))}
            </div>
          </Link>

          <div className="hidden min-w-0 flex-1 md:flex md:items-center md:justify-center">
            {isSignedIn ? (
              <ExternalQuickLinksBar
                compact
                links={headerQuickLinks}
                manageHref={isOrgAdmin ? "/parametres?tab=dashboard-links" : null}
              />
            ) : (
              <Link
                href="/"
                className="rounded-full border border-slate-200 bg-white px-4 py-1.5 text-xs font-bold text-slate-700 transition hover:border-slate-400 hover:text-slate-900"
              >
                Accueil
              </Link>
            )}
          </div>

          <div className="ml-auto flex shrink-0 items-center gap-2">
            {isSignedIn && !isDashboard ? (
              <Link
                href="/dashboard"
                className="hidden rounded-full border border-slate-200 bg-white px-3 py-1.5 text-xs font-bold text-slate-700 transition hover:border-slate-400 hover:text-slate-900 md:inline-flex"
              >
                Dashboard
              </Link>
            ) : null}

            {isSignedIn ? (
              <div ref={popoverRef} className="relative hidden md:block">
                <button
                  onClick={() => setPopoverOpen((v) => !v)}
                  className={`flex h-9 w-9 items-center justify-center rounded-full shadow-sm transition-all ${
                    popoverOpen
                      ? "bg-blue-600 text-white"
                      : "bg-slate-100 text-slate-600 hover:bg-blue-600 hover:text-white"
                  }`}
                  title="Mon compte"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" className="h-4 w-4">
                    <path
                      fillRule="evenodd"
                      d="M7.5 6a4.5 4.5 0 1 1 9 0 4.5 4.5 0 0 1-9 0ZM3.751 20.105a8.25 8.25 0 0 1 16.498 0 .75.75 0 0 1-.437.695A18.683 18.683 0 0 1 12 22.5c-2.786 0-5.433-.608-7.812-1.7a.75.75 0 0 1-.437-.695Z"
                      clipRule="evenodd"
                    />
                  </svg>
                </button>
                {popoverOpen ? (
                  <UserPopover
                    onClose={() => setPopoverOpen(false)}
                    onOpenSecurity={() => setSecurityOpen(true)}
                  />
                ) : null}
              </div>
            ) : (
              <Link
                href="/auth/sign-in"
                className="hidden h-9 w-9 items-center justify-center rounded-full bg-slate-100 text-slate-600 transition-all hover:bg-blue-600 hover:text-white md:flex"
                title="Se connecter"
              >
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" className="h-4 w-4">
                  <path
                    fillRule="evenodd"
                    d="M7.5 6a4.5 4.5 0 1 1 9 0 4.5 4.5 0 0 1-9 0ZM3.751 20.105a8.25 8.25 0 0 1 16.498 0 .75.75 0 0 1-.437.695A18.683 18.683 0 0 1 12 22.5c-2.786 0-5.433-.608-7.812-1.7a.75.75 0 0 1-.437-.695Z"
                    clipRule="evenodd"
                  />
                </svg>
              </Link>
            )}

            <button
              className="relative z-50 flex h-10 w-10 flex-col items-center justify-center gap-[5px] md:hidden"
              onClick={() => setMobileOpen((v) => !v)}
              aria-label={mobileOpen ? "Fermer le menu" : "Ouvrir le menu"}
            >
              <span
                className="block h-[1.5px] rounded-full bg-slate-800 transition-all duration-300 origin-center"
                style={{ width: 24, transform: mobileOpen ? "translateY(6.5px) rotate(45deg)" : "none" }}
              />
              <span
                className="block h-[1.5px] rounded-full bg-slate-800 transition-all duration-300"
                style={{ width: 16, opacity: mobileOpen ? 0 : 1, transform: mobileOpen ? "scaleX(0)" : "none" }}
              />
              <span
                className="block h-[1.5px] rounded-full bg-slate-800 transition-all duration-300 origin-center"
                style={{ width: 24, transform: mobileOpen ? "translateY(-6.5px) rotate(-45deg)" : "none" }}
              />
            </button>
          </div>
        </div>
      </header>

      <div
        className="fixed inset-0 z-40 transition-all duration-500 print:!hidden md:hidden"
        style={{
          background: "rgba(0,0,0,0.18)",
          backdropFilter: "blur(4px)",
          opacity: mobileOpen ? 1 : 0,
          pointerEvents: mobileOpen ? "auto" : "none",
        }}
        onClick={() => setMobileOpen(false)}
      />
      <div
        className="fixed top-14 right-0 left-0 z-40 border-b border-slate-100 bg-white/96 shadow-2xl backdrop-blur-2xl print:!hidden md:hidden"
        style={{
          transform: mobileOpen ? "translateY(0)" : "translateY(-12px)",
          opacity: mobileOpen ? 1 : 0,
          pointerEvents: mobileOpen ? "auto" : "none",
          transition: "transform 0.48s cubic-bezier(0.32,0.72,0,1), opacity 0.3s ease",
        }}
      >
        <div className="mx-auto max-w-[1400px] px-6 pt-4 pb-8">
          <nav className="flex flex-col">
            {isSignedIn ? (
              <>
                <Link
                  href="/dashboard"
                  className={`group flex items-center justify-between border-b border-slate-100 py-4 text-[1.35rem] font-black tracking-tight transition-all duration-300 hover:text-slate-600 ${
                    isDashboard ? "text-slate-800" : "text-slate-900"
                  }`}
                >
                  <span className="flex items-center gap-3">
                    {isDashboard ? <span className="h-2 w-2 flex-shrink-0 rounded-full bg-slate-600" /> : null}
                    Tableau de bord
                  </span>
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2.5} stroke="currentColor" className="h-4 w-4 opacity-30 transition group-hover:opacity-60">
                    <path strokeLinecap="round" strokeLinejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                  </svg>
                </Link>
                {MOBILE_MODULE_LINKS.map((item) => {
                  const linkPath = item.href.split("?")[0].split("#")[0];
                  const active = pathname === linkPath || pathname.startsWith(`${linkPath}/`);
                  return (
                    <Link
                      key={item.href}
                      href={item.href}
                      className={`group flex items-center justify-between border-b border-slate-100 py-3.5 text-[1.1rem] font-bold tracking-tight transition-all duration-300 hover:text-slate-600 ${
                        active ? "text-slate-800" : "text-slate-900"
                      }`}
                    >
                      <span className="flex items-center gap-3">
                        {active ? <span className="h-2 w-2 flex-shrink-0 rounded-full bg-slate-600" /> : null}
                        <span>{item.icon}</span>
                        {item.label}
                      </span>
                      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2.5} stroke="currentColor" className="h-4 w-4 opacity-30 transition group-hover:opacity-60">
                        <path strokeLinecap="round" strokeLinejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                      </svg>
                    </Link>
                  );
                })}
              </>
            ) : (
              <Link
                href="/"
                className={`group flex items-center justify-between border-b border-slate-100 py-4 text-[1.35rem] font-black tracking-tight transition-all duration-300 hover:text-slate-600 ${
                  pathname === "/" ? "text-slate-800" : "text-slate-900"
                }`}
              >
                <span className="flex items-center gap-3">
                  {pathname === "/" ? <span className="h-2 w-2 flex-shrink-0 rounded-full bg-slate-600" /> : null}
                  Accueil
                </span>
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2.5} stroke="currentColor" className="h-4 w-4 opacity-30 transition group-hover:opacity-60">
                  <path strokeLinecap="round" strokeLinejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
                </svg>
              </Link>
            )}
          </nav>
          <div className="mt-5 flex flex-col gap-3">
            {isSignedIn ? (
              <>
                <button
                  type="button"
                  onClick={() => {
                    setMobileOpen(false);
                    setSecurityOpen(true);
                  }}
                  className="w-full rounded-2xl bg-slate-100 py-3.5 text-center text-sm font-bold text-slate-700 transition hover:bg-slate-200"
                >
                  E-mail & mot de passe
                </button>
                <button
                  type="button"
                  onClick={() => signOutWithPortalReset("/")}
                  className="w-full rounded-2xl bg-red-50 py-3.5 text-center text-sm font-bold text-red-500 transition hover:bg-red-100"
                >
                  Se déconnecter
                </button>
              </>
            ) : (
              <Link
                href="/auth/sign-in"
                className="rounded-2xl bg-slate-100 py-3.5 text-center text-sm font-bold text-slate-700 transition hover:bg-slate-200"
              >
                Se connecter
              </Link>
            )}
          </div>
        </div>
      </div>

      <AccountSecurityDialog open={securityOpen} onClose={() => setSecurityOpen(false)} />
    </>
  );
}
