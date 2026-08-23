"use client";

import { useEffect } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { usePathname, useSearchParams } from "next/navigation";
import { hasMasterRole } from "@/app/lib/intranet-role-utils";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { platformAppOrigin } from "@/app/lib/platform-portal-url";
import { resolveEstablishmentPortalOrigin } from "@/app/lib/tenant-portal-client";

function isPlatformMarketingHost(): boolean {
  const host = window.location.hostname.toLowerCase().replace(/^www\./, "");
  try {
    const platformHost = new URL(platformAppOrigin()).hostname.toLowerCase().replace(/^www\./, "");
    return host === platformHost;
  } catch {
    return false;
  }
}

/** Évite l'intranet établissement sur docslapro.com : renvoie vers le sous-domaine mémorisé. */
export default function TenantCanonicalHostGuard() {
  const { isLoaded, isSignedIn, user } = useSessionUser();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  useEffect(() => {
    if (!isLoaded || !isSignedIn || !isPlatformMarketingHost()) return;

    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (hasMasterRole(roles)) return;

    let cancelled = false;
    (async () => {
      const origin = await resolveEstablishmentPortalOrigin();
      if (cancelled || !origin) return;

      if (origin !== window.location.origin) {
        const qs = searchParams.toString();
        window.location.replace(`${origin}${pathname}${qs ? `?${qs}` : ""}`);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [isLoaded, isSignedIn, user, pathname, searchParams]);

  return null;
}
