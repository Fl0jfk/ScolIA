"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";
import Logo from "../../public/Logo header.png";
import {
  usePublicSiteIdentity,
  type PublicSiteIdentity,
} from "@/app/contexts/public-site-identity";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { resolveEstablishmentLogoHomeHref } from "@/app/lib/channel-access";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";

const NAV = [
  { href: "/rentree", label: "Rentrée" },
  { href: "/simulateurFournitures", label: "Simulateur fournitures" },
];

function PublicHeaderLogo({
  identity,
  ready,
  homeHref,
}: {
  identity: PublicSiteIdentity | null;
  ready: boolean;
  homeHref: string;
}) {
  const logoAlt = identity?.shortName || identity?.name || "La Providence Nicolas Barré";
  const customLogoUrl = identity?.headerLogoUrl?.trim() || "";

  return (
    <Link href={homeHref} className="hover:opacity-75 transition flex-shrink-0 relative">
      <div className="w-[110px] h-[110px]">
        {!ready ? (
          <div className="absolute top-7 left-[-20px] sm:left-0 h-[80px] w-[80px]" aria-hidden />
        ) : customLogoUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={customLogoUrl}
            alt={logoAlt}
            className="absolute top-7 left-[-20px] sm:left-0 max-h-[80px] w-auto object-contain [image-rendering:auto]"
          />
        ) : (
          <Image src={Logo} alt={logoAlt} width={300} height={300} className="absolute top-7 left-[-20px] sm:left-0" />
        )}
      </div>
    </Link>
  );
}

export default function RentreePublicHeader() {
  const pathname = usePathname();
  const { isLoaded, isSignedIn, user } = useSessionUser();
  const contextIdentity = usePublicSiteIdentity();
  const [fetchedIdentity, setFetchedIdentity] = useState<PublicSiteIdentity | null>(null);
  const [fetchDone, setFetchDone] = useState(false);

  useEffect(() => {
    if (contextIdentity) return;
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/site/public", { cache: "no-store" });
        const data = (await res.json()) as PublicSiteIdentity;
        if (!cancelled && res.ok) setFetchedIdentity(data);
      } catch {
        /* repli logo par défaut */
      } finally {
        if (!cancelled) setFetchDone(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [contextIdentity]);

  const siteIdentity = contextIdentity ?? fetchedIdentity;
  const logoReady = Boolean(contextIdentity) || fetchDone;

  const homeHref = useMemo(() => {
    if (!isLoaded) return "/rentree";
    const roles = rolesFromUserLike(user);
    const meta = user?.publicMetadata;
    return resolveEstablishmentLogoHomeHref({
      isSignedIn,
      roles,
      orgAdmin: Boolean(meta?.org_admin),
      platformAdmin: Boolean(meta?.platform_admin),
    });
  }, [isLoaded, isSignedIn, user]);

  return (
    <header className="bg-white/90 backdrop-blur-md sticky top-0 z-50 border-b border-slate-100">
      <div className="max-w-[1400px] mx-auto px-6 h-14 flex items-center justify-between gap-4">
        <PublicHeaderLogo identity={siteIdentity} ready={logoReady} homeHref={homeHref} />

        <nav className="hidden md:flex gap-6 text-sm font-bold text-slate-600">
          {NAV.map(({ href, label }) => {
            const active = pathname === href || pathname.startsWith(`${href}/`);
            return (
              <Link
                key={href}
                href={href}
                className={`py-1 transition-colors hover:text-blue-600 ${active ? "text-blue-600" : ""}`}
              >
                {label}
              </Link>
            );
          })}
        </nav>
      </div>
    </header>
  );
}
