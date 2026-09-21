"use client";

import { useEffect, useState } from "react";
import { authClient } from "@/app/lib/auth-client";
import { clearBootstrapCache } from "@/app/lib/app-bootstrap-cache";
import { clearDashboardLinksCache } from "@/app/lib/dashboard-links-cache";
import { clearDashboardSignalsCache } from "@/app/lib/dashboard-signals-cache";
import { clearOnboardingStatusCache } from "@/app/lib/onboarding-status-cache";
import { clearLastPortalTenant } from "@/app/lib/tenant-portal-client";

/**
 * Route `/sign-out` — utilisée par la coque famille / app-mobile (`href="/sign-out"`).
 * Déconnexion Better-Auth + purge caches client, puis redirection sign-in.
 */
export default function SignOutPage() {
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    clearLastPortalTenant();
    clearBootstrapCache();
    clearDashboardLinksCache();
    clearDashboardSignalsCache();
    clearOnboardingStatusCache();

    const params = new URLSearchParams(window.location.search);
    const tenant = params.get("dev_tenant");
    const next = new URL("/auth/sign-in", window.location.origin);
    if (tenant) next.searchParams.set("dev_tenant", tenant);
    else next.searchParams.set("dev_tenant", "default");

    void authClient
      .signOut({
        fetchOptions: {
          onSuccess: () => {
            if (!cancelled) window.location.replace(next.toString());
          },
          onError: () => {
            if (!cancelled) window.location.replace(next.toString());
          },
        },
      })
      .catch(() => {
        if (!cancelled) {
          setError("Déconnexion impossible — redirection…");
          window.location.replace(next.toString());
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="min-h-screen flex items-center justify-center bg-slate-50 px-4">
      <p className="text-sm font-medium text-slate-600">
        {error ?? "Déconnexion en cours…"}
      </p>
    </main>
  );
}
