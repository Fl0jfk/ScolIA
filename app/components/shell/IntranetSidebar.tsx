"use client";

import { useMemo, useState } from "react";
import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useAdminBootstrap } from "@/app/contexts/admin-bootstrap";
import { useData } from "@/app/contexts/data";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";
import { useSignOutWithPortalReset } from "@/app/hooks/useSignOutWithPortalReset";
import AccountSecurityDialog from "@/app/components/Header/AccountSecurityDialog";
import GlobalEleveSearch from "@/app/components/shell/GlobalEleveSearch";
import GlobalDocsDropZone from "@/app/components/shell/GlobalDocsDropZone";
import SidebarScoliaBlock from "@/app/components/shell/SidebarScoliaBlock";
import { useEleveDossierModal } from "@/app/components/shell/EleveDossierModalProvider";
import { QuickLinkIcon } from "@/app/components/Dashboard/ExternalQuickLinks";
import { useMessagingConversations, useMessagingStream } from "@/app/components/messaging/useMessagingData";
import {
  DASHBOARD_PILLARS,
  pillarHasVisibleModules,
} from "@/app/lib/dashboard-pillars";
import { toDashboardQuickLinks } from "@/app/lib/dashboard-quick-links";
import {
  hasGlobalAdminRole,
  intranetRolesFromMetadata,
  rolesFromUserLike,
} from "@/app/lib/intranet-roles";
import { resolveEstablishmentLogoHomeHref } from "@/app/lib/channel-access";
import Logo from "../../../../public/Logo header.png";

const PILLAR_ICONS: Record<string, string> = {
  administratif: "📁",
  etablissement: "🏫",
  services: "🛠️",
  vie_scolaire: "📅",
  compta_rh: "💼",
  sante: "🩺",
};

type Props = {
  mobileOpen: boolean;
  onCloseMobile: () => void;
};

export default function IntranetSidebar({ mobileOpen, onCloseMobile }: Props) {
  const pathname = usePathname();
  const { isSignedIn, user, isLoaded } = useSessionUser();
  const { sitePublic: siteIdentity, loading: bootstrapLoading } = useAdminBootstrap();
  const data = useData();
  const isOrgAdmin = useIsOrgAdmin();
  const signOutWithPortalReset = useSignOutWithPortalReset();
  const { open: openEleveModal } = useEleveDossierModal();
  const [securityOpen, setSecurityOpen] = useState(false);
  const [profileOpen, setProfileOpen] = useState(false);

  const homeHref = resolveEstablishmentLogoHomeHref({
    isSignedIn: Boolean(isLoaded && isSignedIn),
    roles: rolesFromUserLike(user),
    orgAdmin: isOrgAdmin,
    platformAdmin: Boolean(user?.publicMetadata?.platform_admin),
    publicFallback: "/dashboard",
  });

  const logoAlt = siteIdentity?.shortName || siteIdentity?.name || "Établissement";
  const customLogoUrl = siteIdentity?.headerLogoUrl?.trim() || "";
  const tenantName = siteIdentity?.shortName || siteIdentity?.name || "Intranet";

  const userRoles = useMemo(() => {
    if (!user) return [];
    return intranetRolesFromMetadata(user.publicMetadata);
  }, [user]);

  const accessible = data.accessibleModuleIds;
  const canSeeModule = (moduleId: string) => {
    if (isOrgAdmin || hasGlobalAdminRole(userRoles)) return true;
    if (accessible) return accessible.has(moduleId);
    return true;
  };

  const visiblePillars = useMemo(() => {
    if (!isLoaded || !user || !data?.categories) return [];
    const categories = data.categories.filter((c) => c.moduleId !== "dashboard-week-sheet");
    return DASHBOARD_PILLARS.filter((p) =>
      pillarHasVisibleModules(p, categories, userRoles, {
        orgAdmin: isOrgAdmin || hasGlobalAdminRole(userRoles),
        accessibleModuleIds: data.accessibleModuleIds ?? undefined,
      }),
    );
  }, [isLoaded, user, data, userRoles, isOrgAdmin]);

  const quickLinks = useMemo(() => {
    if (!isLoaded || !isSignedIn || !user || !data?.externalQuickLinks) return [];
    const filtered = isOrgAdmin
      ? data.externalQuickLinks
      : data.externalQuickLinks.filter((l) =>
          (l.allowedRoles ?? []).some((r) => userRoles.includes(r)),
        );
    return toDashboardQuickLinks(filtered);
  }, [isLoaded, isSignedIn, user, data, isOrgAdmin, userRoles]);

  const showMessagerie = canSeeModule("messagerie");
  const showChannels = canSeeModule("channels");
  const showOcr = canSeeModule("agent-ia-ocr");

  const messagingEnabled = Boolean(
    isLoaded && isSignedIn && user && (showMessagerie || showChannels),
  );
  const { totalUnread, refresh: refreshMessaging } = useMessagingConversations(messagingEnabled);
  useMessagingStream({
    enabled: messagingEnabled,
    onEvent: (ev) => {
      if (
        ev.type === "message" ||
        ev.type === "conversation_updated" ||
        ev.type === "read"
      ) {
        void refreshMessaging();
      }
    },
    onFallbackPoll: () => void refreshMessaging(),
  });

  function navActive(href: string) {
    if (href === "/dashboard") {
      return pathname === "/dashboard" || pathname.startsWith("/dashboard/");
    }
    if (href === "/messagerie") {
      return (
        pathname === "/messagerie" ||
        pathname.startsWith("/messagerie/") ||
        pathname === "/channels" ||
        pathname.startsWith("/channels/")
      );
    }
    return pathname === href || pathname.startsWith(`${href}/`);
  }

  return (
    <>
      <button
        type="button"
        className={`fixed inset-0 z-40 bg-black/35 transition-opacity lg:hidden ${
          mobileOpen ? "opacity-100" : "pointer-events-none opacity-0"
        }`}
        aria-label="Fermer le menu"
        onClick={onCloseMobile}
      />

      <aside
        className={`fixed inset-y-0 left-0 z-50 flex w-[17.5rem] flex-col border-r border-black/6 bg-[#eceeea] transition-transform duration-300 print:!hidden ${
          mobileOpen ? "translate-x-0" : "-translate-x-full lg:translate-x-0"
        }`}
      >
        <div className="flex items-center gap-3 px-4 pb-3 pt-4">
          <Link
            href={homeHref}
            onClick={onCloseMobile}
            className="flex min-w-0 flex-1 items-center gap-2.5 transition hover:opacity-90"
          >
            <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-white shadow-sm ring-1 ring-black/5">
              {!bootstrapLoading &&
                (customLogoUrl ? (
                  <Image
                    src={customLogoUrl}
                    alt={logoAlt}
                    width={40}
                    height={40}
                    unoptimized
                    className="h-full w-full object-contain p-1"
                  />
                ) : (
                  <Image src={Logo} alt={logoAlt} width={36} height={36} className="object-contain" />
                ))}
            </div>
            <div className="min-w-0">
              <p className="truncate text-sm font-bold tracking-tight text-[var(--dash-ink)]">
                {tenantName}
              </p>
              <p className="truncate text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--dash-mid)]">
                Intranet
              </p>
            </div>
          </Link>
          <button
            type="button"
            className="inline-flex h-8 w-8 items-center justify-center rounded-full bg-white text-[var(--dash-ink)] shadow-sm lg:hidden"
            onClick={onCloseMobile}
            aria-label="Fermer le menu"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth={2}
              stroke="currentColor"
              className="h-4 w-4"
              aria-hidden
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="space-y-4 px-3 pb-3">
          <GlobalEleveSearch
            onSelect={(id) => {
              openEleveModal(id);
              onCloseMobile();
            }}
          />
          <GlobalDocsDropZone onCloseMobile={onCloseMobile} ocrAvailable={showOcr} />
        </div>

        <nav className="flex-1 space-y-1 overflow-y-auto px-3 pb-3">
          <Link
            href="/dashboard"
            onClick={onCloseMobile}
            className={`flex items-center gap-2.5 rounded-2xl px-3 py-2.5 text-sm font-semibold transition ${
              navActive("/dashboard")
                ? "bg-[var(--dash-ink)] text-white shadow-sm"
                : "text-[var(--dash-ink)] hover:bg-white/70"
            }`}
          >
            <span aria-hidden>🏠</span>
            Accueil
          </Link>

          <p className="px-3 pb-1 pt-3 text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--dash-mid)]">
            Espaces
          </p>
          {visiblePillars.map((p) => {
            const active = navActive(p.href);
            return (
              <Link
                key={p.id}
                href={p.href}
                onClick={onCloseMobile}
                className={`flex items-center gap-2.5 rounded-2xl px-3 py-2.5 text-sm font-semibold transition ${
                  active
                    ? "bg-[var(--dash-ink)] text-white shadow-sm"
                    : "text-[var(--dash-ink)] hover:bg-white/70"
                }`}
              >
                <span aria-hidden>{PILLAR_ICONS[p.id] || "•"}</span>
                <span className="min-w-0 flex-1 truncate">{p.title}</span>
                {active ? (
                  <span className="h-2 w-2 shrink-0 rounded-full bg-[var(--dash-lime)]" aria-hidden />
                ) : null}
              </Link>
            );
          })}

          {showMessagerie || showChannels ? (
            <Link
              href={showMessagerie ? "/messagerie" : "/channels"}
              onClick={onCloseMobile}
              className={`flex items-center gap-2.5 rounded-2xl px-3 py-2.5 text-sm font-semibold transition ${
                navActive("/messagerie")
                  ? "bg-[var(--dash-ink)] text-white shadow-sm"
                  : "text-[var(--dash-ink)] hover:bg-white/70"
              }`}
            >
              <span aria-hidden>💬</span>
              <span className="min-w-0 flex-1 truncate">Messagerie</span>
              {totalUnread > 0 ? (
                <span
                  className="flex h-5 min-w-[1.25rem] items-center justify-center rounded-full bg-[var(--dash-lime)] px-1.5 text-[10px] font-black text-[var(--dash-ink)]"
                  title={`${totalUnread} non lu${totalUnread > 1 ? "s" : ""}`}
                >
                  {totalUnread > 99 ? "99+" : totalUnread}
                </span>
              ) : navActive("/messagerie") ? (
                <span className="h-2 w-2 shrink-0 rounded-full bg-[var(--dash-lime)]" aria-hidden />
              ) : null}
            </Link>
          ) : null}

          {(quickLinks.length > 0 || isOrgAdmin) && (
            <>
              <p className="flex items-center justify-between gap-2 px-3 pb-1 pt-3 text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--dash-mid)]">
                <span>Raccourcis</span>
                {isOrgAdmin ? (
                  <Link
                    href="/parametres?tab=dashboard-links"
                    onClick={onCloseMobile}
                    className="rounded-full bg-[var(--dash-ink)] px-1.5 py-0.5 text-[9px] font-black text-white"
                    title="Gérer les raccourcis"
                  >
                    +
                  </Link>
                ) : null}
              </p>
              {quickLinks.map((link) => (
                <a
                  key={link.id}
                  href={link.link}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={onCloseMobile}
                  title={link.name}
                  className="flex items-center gap-2.5 rounded-2xl px-3 py-2 text-sm font-medium text-neutral-700 transition hover:bg-white/70 hover:text-[var(--dash-ink)]"
                >
                  <QuickLinkIcon src={link.img} name={link.name} />
                  <span className="min-w-0 flex-1 truncate">{link.name}</span>
                </a>
              ))}
              {quickLinks.length === 0 && isOrgAdmin ? (
                <Link
                  href="/parametres?tab=dashboard-links"
                  onClick={onCloseMobile}
                  className="rounded-2xl px-3 py-2 text-xs font-medium text-neutral-500 hover:bg-white/70"
                >
                  Ajouter des raccourcis…
                </Link>
              ) : null}
            </>
          )}
        </nav>

        <div className="space-y-2 border-t border-black/6 p-3">
          <SidebarScoliaBlock onCloseMobile={onCloseMobile} />

          <div className="relative">
            <button
              type="button"
              onClick={() => setProfileOpen((v) => !v)}
              className="flex w-full items-center gap-3 rounded-2xl bg-white/80 px-3 py-2.5 text-left shadow-sm ring-1 ring-black/5 transition hover:bg-white"
            >
              {user?.imageUrl ? (
                <img
                  src={user.imageUrl}
                  alt=""
                  className="h-9 w-9 shrink-0 rounded-full object-cover"
                />
              ) : (
                <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-[var(--dash-lime)] text-sm font-black text-[var(--dash-ink)]">
                  {(user?.firstName?.[0] || user?.fullName?.[0] || "?").toUpperCase()}
                </div>
              )}
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-bold text-[var(--dash-ink)]">
                  {user?.fullName || user?.username || "Compte"}
                </p>
                <p className="truncate text-[11px] text-neutral-500">
                  {user?.primaryEmailAddress?.emailAddress || "Réglages"}
                </p>
              </div>
            </button>

            {profileOpen ? (
              <div className="absolute bottom-[calc(100%+0.35rem)] left-0 right-0 z-20 overflow-hidden rounded-2xl border border-black/8 bg-white shadow-xl">
                <Link
                  href="/parametres"
                  onClick={() => {
                    setProfileOpen(false);
                    onCloseMobile();
                  }}
                  className="block px-4 py-3 text-sm font-semibold text-[var(--dash-ink)] hover:bg-[color:var(--dash-soft-muted)]"
                >
                  Paramètres
                </Link>
                <button
                  type="button"
                  onClick={() => {
                    setSecurityOpen(true);
                    setProfileOpen(false);
                  }}
                  className="block w-full px-4 py-3 text-left text-sm font-semibold text-[var(--dash-ink)] hover:bg-[color:var(--dash-soft-muted)]"
                >
                  Sécurité
                </button>
                <button
                  type="button"
                  onClick={() => {
                    signOutWithPortalReset("/");
                    setProfileOpen(false);
                  }}
                  className="block w-full border-t border-black/6 px-4 py-3 text-left text-sm font-semibold text-red-600 hover:bg-red-50"
                >
                  Se déconnecter
                </button>
              </div>
            ) : null}
          </div>
        </div>

        <AccountSecurityDialog open={securityOpen} onClose={() => setSecurityOpen(false)} />
      </aside>
    </>
  );
}
