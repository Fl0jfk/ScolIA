"use client";

import { useCallback, useEffect, useState } from "react";
import Image from "next/image";
import MarketingShell from "@/app/components/landing/MarketingShell";
import ConnexionPlatformSessionBanner from "@/app/components/ConnexionPlatformSessionBanner";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";
import {
  catalogEntrySignInUrl,
  clearLastPortalTenant,
  readLastPortalTenant,
  saveLastPortalTenant,
  syncSavedPortalTenantFromCatalog,
} from "@/app/lib/tenant-portal-client";
import { isBrowserLocalDev } from "@/app/lib/local-dev";
import { platformAdminSignInUrl } from "@/app/lib/platform-portal-url";

type TenantEntry = {
  slug: string;
  kind: string;
  kindLabel: string;
  label: string;
  postalAddressLabel: string;
  logoUrl: string | null;
  signInUrl: string;
  primaryHostname: string | null;
  appUrl: string;
};

function goToTenantSignIn(tenant: TenantEntry, signInHref: string) {
  saveLastPortalTenant({
    slug: tenant.slug,
    label: tenant.label,
    signInUrl: catalogEntrySignInUrl(tenant),
  });

  try {
    const targetOrigin = new URL(signInHref).origin;
    if (targetOrigin !== window.location.origin) {
      window.location.assign(signInHref);
      return;
    }
  } catch {
    /* fall through */
  }

  window.location.assign(signInHref);
}

export default function ConnexionPage() {
  const [tenants, setTenants] = useState<TenantEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [redirecting, setRedirecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isLocalDev, setIsLocalDev] = useState(false);

  useEffect(() => {
    setIsLocalDev(isBrowserLocalDev());
  }, []);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const res = await fetch("/api/tenants/public", { cache: "no-store" });
        const j = await res.json();
        if (!res.ok) throw new Error(j.error || "Chargement impossible");

        const list = (j.tenants ?? []) as TenantEntry[];
        if (cancelled) return;

        const savedBeforeFetch = readLastPortalTenant();
        if (savedBeforeFetch?.slug) {
          setRedirecting(true);
          const refreshed = syncSavedPortalTenantFromCatalog(list);
          if (refreshed?.signInUrl) {
            const hit = list.find((t) => t.slug === refreshed.slug);
            if (hit) {
              goToTenantSignIn(hit, catalogEntrySignInUrl(hit));
              return;
            }
          }
          clearLastPortalTenant();
          setRedirecting(false);
        }

        setTenants(list);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Erreur");
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const handleChoose = useCallback((tenant: TenantEntry) => {
    goToTenantSignIn(tenant, catalogEntrySignInUrl(tenant));
  }, []);

  const adminSignInHref = isLocalDev
    ? `/auth/sign-in?redirect_url=${encodeURIComponent("/plateforme")}`
    : platformAdminSignInUrl();

  if (redirecting) {
    return (
      <MarketingShell>
        <main className="mx-auto max-w-lg px-6 py-24 text-center">
          <p className="text-sm font-medium text-stone-600">Redirection vers votre établissement…</p>
        </main>
      </MarketingShell>
    );
  }

  return (
    <MarketingShell>
      <main className="mx-auto max-w-3xl px-6 py-12">
        <div className="text-center">
          <h1 className="text-3xl font-black text-[#14231A]">
            Connexion à votre <span className={SCOLA_GRADIENT_TEXT}>intranet</span>
          </h1>
          <p className="mx-auto mt-3 max-w-md text-sm text-stone-600">
            Choisissez votre établissement pour accéder à l’intranet.
          </p>
        </div>

        <ConnexionPlatformSessionBanner />

        {loading && <p className="mt-12 text-center text-sm text-stone-500">Chargement des établissements…</p>}
        {error && <p className="mt-12 text-center text-sm text-red-600">{error}</p>}

        {!loading && !error && tenants.length > 0 && (
          <ul className="mt-10 grid gap-4 sm:grid-cols-2">
            {tenants.map((tenant) => (
              <li key={tenant.slug}>
                <button
                  type="button"
                  onClick={() => handleChoose(tenant)}
                  className="group flex w-full flex-col items-center gap-3 rounded-2xl border border-stone-200/90 bg-white px-5 py-6 text-center shadow-sm transition hover:-translate-y-0.5 hover:border-[#2F6B4A]/40 hover:shadow-md focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#2F6B4A]"
                >
                  {tenant.logoUrl ? (
                    <div className="flex h-24 w-24 items-center justify-center rounded-2xl bg-stone-50 p-3 shadow-inner ring-1 ring-stone-100 transition group-hover:ring-[#2F6B4A]/30">
                      <Image
                        src={tenant.logoUrl}
                        alt=""
                        width={88}
                        height={88}
                        className="h-full w-full object-contain"
                        unoptimized
                      />
                    </div>
                  ) : (
                    <div className="flex h-24 w-24 items-center justify-center rounded-2xl bg-gradient-to-br from-emerald-50 to-emerald-100 text-3xl font-black text-[#2F6B4A] shadow-inner">
                      {tenant.label.charAt(0)}
                    </div>
                  )}
                  <div>
                    <p className="text-base font-bold text-[#14231A]">{tenant.label}</p>
                    <p className="mt-1 text-xs font-semibold uppercase tracking-wide text-[#2F6B4A]/80">
                      {tenant.kindLabel}
                    </p>
                    {tenant.postalAddressLabel ? (
                      <p className="mt-2 text-sm text-stone-600">{tenant.postalAddressLabel}</p>
                    ) : null}
                  </div>
                  <span className="mt-1 rounded-full bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-1.5 text-xs font-bold text-white opacity-90 transition group-hover:opacity-100">
                    Se connecter
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}

        {!loading && !error && tenants.length === 0 && (
          <p className="mt-12 text-center text-sm text-stone-500">Aucun établissement disponible.</p>
        )}

        <p className="mt-12 text-center text-xs text-stone-500">
          Vous gérez la plateforme Scola ?{" "}
          <a href={adminSignInHref} className="font-semibold text-violet-700 hover:underline">
            Administration
          </a>
        </p>
      </main>
    </MarketingShell>
  );
}
