"use client";

import { useClerk } from "@clerk/nextjs";
import { useCallback } from "react";
import { clearBootstrapCache } from "@/app/lib/app-bootstrap-cache";
import { clearDashboardLinksCache } from "@/app/lib/dashboard-links-cache";
import { clearDashboardSignalsCache } from "@/app/lib/dashboard-signals-cache";
import { clearOnboardingStatusCache } from "@/app/lib/onboarding-status-cache";
import { clearLastPortalTenant } from "@/app/lib/tenant-portal-client";

function signOutRedirectUrl(): string {
  if (typeof window === "undefined") return "/";
  try {
    return `${window.location.origin}/`;
  } catch {
    return "/";
  }
}

/** Déconnexion Clerk + oubli du dernier établissement mémorisé sur cet appareil. */
export function useSignOutWithPortalReset() {
  const { signOut } = useClerk();
  return useCallback(
    (redirectUrl?: string) => {
      clearLastPortalTenant();
      clearBootstrapCache();
      clearDashboardLinksCache();
      clearDashboardSignalsCache();
      clearOnboardingStatusCache();
      void signOut({ redirectUrl: redirectUrl ?? signOutRedirectUrl() });
    },
    [signOut],
  );
}
