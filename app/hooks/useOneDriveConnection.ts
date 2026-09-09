"use client";

import { useCallback, useEffect, useState } from "react";
import * as msal from "@azure/msal-browser";
import {
  ONEDRIVE_MSAL_SCOPES,
  buildOneDriveMsalConfig,
  fetchMicrosoftOneDrivePublicConfig,
  storeMsalReturnPath,
} from "@/app/lib/msal-onedrive-client";
import {
  obtainValidOneDriveAccessToken,
  pickCachedAccessToken,
  tryRestoreOneDriveAccessToken,
} from "@/app/lib/onedrive-msal-session";

const ONEDRIVE_SCOPES = [...ONEDRIVE_MSAL_SCOPES];

let msalInstance: msal.PublicClientApplication | null = null;

function getMsal() {
  if (!msalInstance) throw new Error("MSAL non initialisé");
  return msalInstance;
}

type OneDriveConnectionState = {
  msalReady: boolean;
  oneDriveEnabled: boolean;
  connected: boolean;
  checking: boolean;
  accountLabel: string | null;
  accessToken: string | null;
  error: string | null;
  login: () => Promise<void>;
  ensureToken: () => Promise<string | null>;
};

/**
 * @param enabled — si false, n'initialise pas MSAL (évite iframes sandbox + appels verify au chargement).
 * @param restoreOnMount — si false, n'appelle pas acquireTokenSilent au chargement
 *   (supprime les warnings Chrome sandbox allow-scripts+allow-same-origin de MSAL).
 */
export function useOneDriveConnection(options?: {
  enabled?: boolean;
  restoreOnMount?: boolean;
}): OneDriveConnectionState {
  const enabled = options?.enabled !== false;
  const restoreOnMount = options?.restoreOnMount !== false;
  const [msalReady, setMsalReady] = useState(false);
  const [oneDriveEnabled, setOneDriveEnabled] = useState(false);
  const [connected, setConnected] = useState(false);
  const [checking, setChecking] = useState(false);
  const [accountLabel, setAccountLabel] = useState<string | null>(null);
  const [accessToken, setAccessToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const applySession = useCallback((account: msal.AccountInfo | null, token: string | null) => {
    setAccessToken(token);
    setConnected(Boolean(token));
    setAccountLabel(account?.username ?? account?.name ?? null);
  }, []);

  useEffect(() => {
    if (!enabled) {
      setMsalReady(false);
      setOneDriveEnabled(false);
      setConnected(false);
      setAccessToken(null);
      setAccountLabel(null);
      setError(null);
      return;
    }

    let cancelled = false;
    (async () => {
      try {
        const ms = await fetchMicrosoftOneDrivePublicConfig();
        if (!ms) {
          if (!cancelled) {
            setOneDriveEnabled(false);
            setMsalReady(true);
          }
          return;
        }
        if (!cancelled) setOneDriveEnabled(true);

        msalInstance = new msal.PublicClientApplication(buildOneDriveMsalConfig(ms));
        await getMsal().initialize();
        await getMsal().handleRedirectPromise({ navigateToLoginRequestUrl: false });
        if (cancelled) return;
        setMsalReady(true);

        if (!restoreOnMount) return;

        const accounts = getMsal().getAllAccounts();
        if (accounts.length > 0) {
          try {
            const token = await tryRestoreOneDriveAccessToken(getMsal(), accounts[0]);
            if (!cancelled) applySession(accounts[0], token);
          } catch {
            if (!cancelled) applySession(accounts[0], null);
          }
        }
      } catch (e: unknown) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : "Erreur init OneDrive");
          setMsalReady(true);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [applySession, enabled, restoreOnMount]);

  const ensureToken = useCallback(async (): Promise<string | null> => {
    if (!enabled) {
      setError("OneDrive non disponible sur cet écran.");
      return null;
    }
    if (!msalReady || !oneDriveEnabled) {
      setError("OneDrive non activé pour cet établissement.");
      return null;
    }
    setChecking(true);
    setError(null);
    try {
      const accounts = getMsal().getAllAccounts();
      if (accounts.length === 0) {
        applySession(null, null);
        setError("Connectez-vous à OneDrive (bouton Connexion Microsoft).");
        return null;
      }

      const cached = pickCachedAccessToken(accessToken);
      if (cached) {
        applySession(accounts[0], cached);
        return cached;
      }

      const token = await obtainValidOneDriveAccessToken(getMsal(), accounts[0]);
      applySession(accounts[0], token);
      return token;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      applySession(getMsal().getAllAccounts()[0] ?? null, null);
      setError(msg);
      return null;
    } finally {
      setChecking(false);
    }
  }, [accessToken, applySession, enabled, msalReady, oneDriveEnabled]);

  const login = useCallback(async () => {
    if (!enabled || !msalReady) return;
    setError(null);
    try {
      storeMsalReturnPath();
      await getMsal().loginRedirect({
        scopes: ONEDRIVE_SCOPES,
        prompt: "select_account",
      });
    } catch (e: unknown) {
      if (e instanceof msal.BrowserAuthError && e.errorCode === "interaction_in_progress") {
        setError("Connexion déjà en cours…");
        return;
      }
      setError(e instanceof Error ? e.message : "Échec connexion OneDrive");
    }
  }, [enabled, msalReady]);

  return {
    msalReady,
    oneDriveEnabled,
    connected,
    checking,
    accountLabel,
    accessToken,
    error,
    login,
    ensureToken,
  };
}
