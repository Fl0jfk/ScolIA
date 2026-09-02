"use client";

import {
  createContext,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import DashboardBootstrapOverlay from "@/app/components/Dashboard/DashboardBootstrapOverlay";
import { useSessionUser } from "@/app/hooks/useAppUser";
import {
  applyDashboardBrandToDocument,
  readCachedDashboardAccent,
  writeCachedDashboardAccent,
} from "@/app/lib/dashboard-accent-cache";
import {
  readBootstrapCache,
  writeBootstrapCache,
} from "@/app/lib/app-bootstrap-cache";
import { parseDashboardAccent } from "@/app/lib/dashboard-brand-presets";
import type {
  DomainPlanningModuleConfig,
  Establishment,
  ProfRoomModuleConfig,
  TravelsModuleConfig,
} from "@/app/lib/app-config-schemas";

export type AppContextPayload = {
  identity: { name: string; shortName?: string; dashboardAccent?: string };
  establishments: Establishment[];
  profRoom?: ProfRoomModuleConfig;
  domainPlanning?: DomainPlanningModuleConfig;
  travelsOptions?: TravelsModuleConfig;
  session?: {
    intranetRoles: string[];
    isGlobalAdmin: boolean;
  };
};

export type SitePublicIdentity = {
  name?: string;
  shortName?: string;
  headerLogoUrl?: string | null;
};

type AdminBootstrapContextValue = {
  appContext: AppContextPayload | null;
  sitePublic: SitePublicIdentity | null;
  loading: boolean;
  error: string | null;
};

export const AdminBootstrapContext = createContext<AdminBootstrapContextValue | null>(null);

function preloadImage(url: string): Promise<void> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve();
    img.onerror = () => resolve();
    img.src = url;
  });
}

export function useAdminBootstrap(): AdminBootstrapContextValue {
  const ctx = useContext(AdminBootstrapContext);
  if (!ctx) {
    throw new Error("useAdminBootstrap doit être utilisé dans AdminBootstrapProvider");
  }
  return ctx;
}

export function AdminBootstrapProvider({
  children,
  enableOverlay = true,
}: {
  children: ReactNode;
  /** Désactiver sur /sign-in pour ne pas masquer le formulaire de connexion. */
  enableOverlay?: boolean;
}) {
  const { isLoaded: sessionLoaded, isSignedIn } = useSessionUser();

  const [appContext, setAppContext] = useState<AppContextPayload | null>(null);
  const [sitePublic, setSitePublic] = useState<SitePublicIdentity | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [accentReady, setAccentReady] = useState(false);
  const [assetsReady, setAssetsReady] = useState(false);
  const [hasWarmCache, setHasWarmCache] = useState(false);
  const shouldBlock =
    enableOverlay && (!sessionLoaded || ((loading || !assetsReady) && !hasWarmCache));
  const [overlayOpen, setOverlayOpen] = useState(enableOverlay);

  useLayoutEffect(() => {
    const cachedAccent = readCachedDashboardAccent();
    if (cachedAccent) {
      applyDashboardBrandToDocument(cachedAccent);
      setAccentReady(true);
    }

    const cachedBootstrap = readBootstrapCache();
    if (!cachedBootstrap) return;

    setSitePublic(cachedBootstrap.sitePublic);
    setAppContext(cachedBootstrap.appContext);
    const accent = parseDashboardAccent(cachedBootstrap.appContext?.identity?.dashboardAccent);
    writeCachedDashboardAccent(accent);
    applyDashboardBrandToDocument(accent);
    setAccentReady(true);
    setLoading(false);
    setAssetsReady(true);
    setHasWarmCache(true);
  }, []);

  useEffect(() => {
    if (!sessionLoaded) return;

    let cancelled = false;
    (async () => {
      try {
        const siteP = fetch("/api/site/public", { cache: "no-store" });
        const ctxP = isSignedIn ? fetch("/api/app/context", { cache: "no-store" }) : null;
        const [siteRes, ctxRes] = await Promise.all([siteP, ctxP ?? Promise.resolve(null)]);
        const siteJson = (await siteRes.json()) as SitePublicIdentity;

        if (!cancelled && siteRes.ok) {
          setSitePublic(siteJson);
          const logoUrl = siteJson.headerLogoUrl?.trim() || "";
          if (logoUrl && !hasWarmCache) {
            await preloadImage(logoUrl);
          } else if (logoUrl) {
            void preloadImage(logoUrl);
          }
        }

        let nextAppContext: AppContextPayload | null = null;
        if (isSignedIn && ctxRes) {
          const ctxJson = (await ctxRes.json()) as AppContextPayload & { error?: string };

          if (!cancelled && ctxRes.ok) {
            nextAppContext = ctxJson;
            setAppContext(ctxJson);
            const accent = parseDashboardAccent(ctxJson.identity?.dashboardAccent);
            writeCachedDashboardAccent(accent);
            applyDashboardBrandToDocument(accent);
            setAccentReady(true);
          } else if (!cancelled && enableOverlay && !hasWarmCache) {
            throw new Error(ctxJson.error || "Contexte indisponible");
          }
        } else if (!cancelled) {
          setAppContext(null);
        }

        if (!cancelled && siteRes.ok) {
          writeBootstrapCache({
            sitePublic: siteJson,
            appContext: isSignedIn ? nextAppContext : null,
          });
        }
      } catch (e) {
        if (!cancelled && !hasWarmCache) setError(e instanceof Error ? e.message : "Erreur");
      } finally {
        if (!cancelled) {
          setLoading(false);
          setAssetsReady(true);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionLoaded, isSignedIn, enableOverlay, hasWarmCache]);

  useEffect(() => {
    if (!enableOverlay) {
      setOverlayOpen(false);
      return;
    }
    setOverlayOpen(shouldBlock);
  }, [enableOverlay, shouldBlock]);

  const value = useMemo(
    () => ({ appContext, sitePublic, loading, error }),
    [appContext, sitePublic, loading, error],
  );

  return (
    <AdminBootstrapContext.Provider value={value}>
      {enableOverlay ? <DashboardBootstrapOverlay open={overlayOpen} accentReady={accentReady} /> : null}
      {children}
    </AdminBootstrapContext.Provider>
  );
}
