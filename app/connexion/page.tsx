"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
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
  tenantSelectLabel,
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
  const [selectedSlug, setSelectedSlug] = useState("");
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

  const selectedTenant = useMemo(
    () => tenants.find((t) => t.slug === selectedSlug) ?? null,
    [tenants, selectedSlug],
  );

  const handleContinue = useCallback(() => {
    if (!selectedTenant) return;
    goToTenantSignIn(selectedTenant, catalogEntrySignInUrl(selectedTenant));
  }, [selectedTenant]);

  const adminSignInHref = isLocalDev
    ? `/sign-in?redirect_url=${encodeURIComponent("/plateforme")}`
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
      <main className="mx-auto max-w-lg px-6 py-12">
        <div className="text-center">
          <h1 className="text-3xl font-black text-[#14231A]">
            Connexion à votre <span className={SCOLA_GRADIENT_TEXT}>intranet</span>
          </h1>
          <p className="mx-auto mt-3 max-w-md text-sm text-stone-600">
            {isLocalDev
              ? "Retrouvez la liste des établissements pour accéder à l’intranet (localhost en développement)."
              : "Retrouvez la liste des établissements pour accéder à l’intranet."}
          </p>
        </div>

        <ConnexionPlatformSessionBanner />

        {loading && <p className="mt-12 text-center text-sm text-stone-500">Chargement des établissements…</p>}
        {error && <p className="mt-12 text-center text-sm text-red-600">{error}</p>}

        {!loading && !error && tenants.length > 0 && (
          <div className="mt-10 space-y-6">
            <label className="block">
              <span className="mb-2 block text-sm font-semibold text-stone-700">Établissement scolaire</span>
              <select
                value={selectedSlug}
                onChange={(event) => setSelectedSlug(event.target.value)}
                className="w-full rounded-xl border border-stone-200 bg-white px-4 py-3 text-sm text-stone-800 shadow-sm outline-none transition focus:border-[#2F6B4A] focus:ring-2 focus:ring-[#2F6B4A]/20"
              >
                <option value="">Sélectionnez votre établissement…</option>
                {tenants.map((tenant) => (
                  <option key={tenant.slug} value={tenant.slug}>
                    {tenantSelectLabel(tenant)}
                  </option>
                ))}
              </select>
            </label>

            {selectedTenant && (
              <div className="flex flex-col items-center rounded-2xl border border-stone-200/80 bg-white px-5 py-6 text-center shadow-sm">
                {selectedTenant.logoUrl ? (
                  <div className="flex h-20 w-20 items-center justify-center rounded-xl bg-stone-50 p-2 shadow-inner">
                    <Image
                      src={selectedTenant.logoUrl}
                      alt=""
                      width={72}
                      height={72}
                      className="h-full w-full object-contain"
                      unoptimized
                    />
                  </div>
                ) : (
                  <div className="flex h-20 w-20 items-center justify-center rounded-xl bg-gradient-to-br from-emerald-50 to-emerald-100 text-2xl font-black text-[#2F6B4A]">
                    {selectedTenant.label.charAt(0)}
                  </div>
                )}
                <p className="mt-4 text-lg font-bold text-[#14231A]">{selectedTenant.label}</p>
                <p className="mt-1 text-sm text-stone-500">{selectedTenant.kindLabel}</p>
                {selectedTenant.postalAddressLabel ? (
                  <p className="mt-2 text-sm text-stone-600">{selectedTenant.postalAddressLabel}</p>
                ) : null}
                {selectedTenant.primaryHostname && (
                  <p className="mt-2 font-mono text-[11px] text-stone-400">{selectedTenant.primaryHostname}</p>
                )}
              </div>
            )}

            <button
              type="button"
              onClick={handleContinue}
              disabled={!selectedTenant}
              className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-5 py-3.5 text-sm font-bold text-white shadow-md transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-50"
            >
              Continuer
            </button>
          </div>
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
