"use client";

import { useCallback } from "react";
import { authClient } from "@/app/lib/auth-client";
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

/** Déconnexion Better-Auth + oubli du dernier établissement mémorisé sur cet appareil. */
export function useSignOutWithPortalReset() {
  return useCallback((redirectUrl?: string) => {
    clearLastPortalTenant();
    clearBootstrapCache();
    clearDashboardLinksCache();
    clearDashboardSignalsCache();
    clearOnboardingStatusCache();
    const target = redirectUrl ?? signOutRedirectUrl();
    void authClient.signOut({
      fetchOptions: {
        onSuccess: () => {
          window.location.href = target;
        },
        onError: () => {
          window.location.href = target;
        },
      },
    });
  }, []);
}
