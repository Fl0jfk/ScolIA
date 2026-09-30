"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import * as msal from "@azure/msal-browser";
import {
  buildOneDriveMsalConfig,
  fetchMicrosoftOneDrivePublicConfig,
  ONEDRIVE_MSAL_SCOPES,
  storeMsalReturnPath,
} from "@/app/lib/msal-onedrive-client";
import {
  obtainValidOneDriveAccessToken,
  pickCachedAccessToken,
  tryRestoreOneDriveAccessToken,
} from "@/app/lib/onedrive-msal-session";

const SCOPES = [...ONEDRIVE_MSAL_SCOPES];

let sharedPca: msal.PublicClientApplication | null = null;

function getPca(): msal.PublicClientApplication {
  if (!sharedPca) throw new Error("MSAL non initialisé");
  return sharedPca;
}

export type OcrSuiviSnapshot = {
  activeCount: number;
  failedRecent: number;
  needsToken: boolean;
};

export type OneDriveOcrGate = {
  ready: boolean;
  checking: boolean;
  /** Au moins un flux OCR (élèves / enseignants / personnel) dans Paramètres. */
  assigned: boolean;
  configured: boolean;
  connected: boolean;
  error: string | null;
  connect: () => Promise<void>;
  ensureConnected: () => Promise<boolean>;
  suivi: OcrSuiviSnapshot;
  refreshSuivi: () => Promise<void>;
};

/**
 * Session OneDrive légère pour le dépôt sidebar + pastille de suivi OCR.
 * N’active le dépôt que si l’utilisateur est nommé sur au moins un flux OCR.
 */
export function useOneDriveOcrGate(enabled: boolean): OneDriveOcrGate {
  const [ready, setReady] = useState(false);
  const [checking, setChecking] = useState(false);
  const [assigned, setAssigned] = useState(false);
  const [configured, setConfigured] = useState(false);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [suivi, setSuivi] = useState<OcrSuiviSnapshot>({
    activeCount: 0,
    failedRecent: 0,
    needsToken: false,
  });
  const tokenRef = useRef<string | null>(null);

  const refreshSuivi = useCallback(async () => {
    if (!enabled) return;
    try {
      const res = await fetch("/api/agentIAOCR/batch-job/list", { cache: "no-store" });
      if (!res.ok) return;
      const data = (await res.json()) as {
        activeCount?: number;
        jobs?: Array<{ status?: string; failed?: number }>;
      };
      const jobs = Array.isArray(data.jobs) ? data.jobs : [];
      const failedRecent = jobs.reduce((acc, j) => acc + (Number(j.failed) || 0), 0);
      const needsToken = jobs.some((j) => j.status === "needs_token");
      setSuivi({
        activeCount: Number(data.activeCount) || 0,
        failedRecent,
        needsToken,
      });
    } catch {
      /* ignore */
    }
  }, [enabled]);

  useEffect(() => {
    if (!enabled) {
      setReady(true);
      setAssigned(false);
      setConfigured(false);
      setConnected(false);
      setChecking(false);
      return;
    }
    let cancelled = false;
    (async () => {
      setChecking(true);
      try {
        const profileRes = await fetch("/api/onedrive/profile", {
          credentials: "include",
          cache: "no-store",
        });
        let hasFlux = false;
        if (profileRes.ok) {
          const j = (await profileRes.json()) as {
            profile?: unknown;
            fluxes?: unknown[];
          };
          const fluxes = Array.isArray(j.fluxes) ? j.fluxes : [];
          hasFlux = fluxes.length > 0 || Boolean(j.profile);
        }
        if (cancelled) return;
        setAssigned(hasFlux);
        if (!hasFlux) {
          setConfigured(false);
          setConnected(false);
          setReady(true);
          return;
        }

        const ms = await fetchMicrosoftOneDrivePublicConfig();
        if (cancelled) return;
        if (!ms) {
          setConfigured(false);
          setError("OneDrive non activé pour cet établissement.");
          setReady(true);
          return;
        }
        if (!sharedPca) {
          sharedPca = new msal.PublicClientApplication(buildOneDriveMsalConfig(ms));
          await sharedPca.initialize();
        }
        const pca = getPca();
        const redirectResult = await pca.handleRedirectPromise({
          navigateToLoginRequestUrl: false,
        });
        if (redirectResult?.account) {
          pca.setActiveAccount(redirectResult.account);
        }
        const account = pca.getActiveAccount() || pca.getAllAccounts()[0] || null;
        if (account) {
          pca.setActiveAccount(account);
          const token = await tryRestoreOneDriveAccessToken(pca, account);
          if (!cancelled) {
            tokenRef.current = token;
            setConnected(Boolean(token));
          }
        }
        if (!cancelled) {
          setConfigured(true);
          setError(null);
          setReady(true);
        }
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : "Init OneDrive impossible");
          setReady(true);
        }
      } finally {
        if (!cancelled) setChecking(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [enabled]);

  useEffect(() => {
    if (!enabled || !assigned) return;
    void refreshSuivi();
    const t = window.setInterval(() => void refreshSuivi(), 20000);
    return () => window.clearInterval(t);
  }, [enabled, assigned, refreshSuivi]);

  const connect = useCallback(async () => {
    if (!ready || !configured || !assigned) return;
    setError(null);
    setChecking(true);
    try {
      const pca = getPca();
      const returnPath =
        typeof window !== "undefined"
          ? `${window.location.pathname}${window.location.search}`
          : "/";
      storeMsalReturnPath(returnPath || "/");
      await pca.loginRedirect({
        scopes: SCOPES,
        prompt: "select_account",
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Connexion OneDrive impossible");
      setChecking(false);
    }
  }, [ready, configured, assigned]);

  const ensureConnected = useCallback(async () => {
    if (!ready || !configured || !assigned) return false;
    const cached = pickCachedAccessToken(tokenRef.current);
    if (cached) {
      setConnected(true);
      return true;
    }
    setChecking(true);
    try {
      const pca = getPca();
      const account = pca.getActiveAccount() || pca.getAllAccounts()[0] || null;
      if (!account) {
        setConnected(false);
        setError("Connectez OneDrive pour déposer des documents.");
        return false;
      }
      pca.setActiveAccount(account);
      const token = await obtainValidOneDriveAccessToken(pca, account);
      tokenRef.current = token;
      setConnected(true);
      setError(null);
      return true;
    } catch (e) {
      setConnected(false);
      setError(e instanceof Error ? e.message : "OneDrive indisponible");
      return false;
    } finally {
      setChecking(false);
    }
  }, [ready, configured, assigned]);

  return {
    ready,
    checking,
    assigned,
    configured,
    connected,
    error,
    connect,
    ensureConnected,
    suivi,
    refreshSuivi,
  };
}
