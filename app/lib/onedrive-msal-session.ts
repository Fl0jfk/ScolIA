"use client";

import * as msal from "@azure/msal-browser";
import { isAccessTokenExpired } from "@/app/lib/onedrive-access-token";
import { ONEDRIVE_MSAL_SCOPES, storeMsalReturnPath } from "@/app/lib/msal-onedrive-client";

const SCOPES = [...ONEDRIVE_MSAL_SCOPES];

/** Une seule acquireTokenSilent à la fois (OCR + sidebar + stages partageaient le même PCA). */
const silentInFlight = new Map<string, Promise<msal.AuthenticationResult>>();

function silentKey(account: msal.AccountInfo, forceRefresh: boolean): string {
  return `${account.homeAccountId}|${forceRefresh ? "1" : "0"}|${SCOPES.join(" ")}`;
}

async function acquireTokenSilentOnce(
  pca: msal.PublicClientApplication,
  account: msal.AccountInfo,
  forceRefresh: boolean,
): Promise<msal.AuthenticationResult> {
  const key = silentKey(account, forceRefresh);
  const existing = silentInFlight.get(key);
  if (existing) return existing;

  const promise = pca
    .acquireTokenSilent({ account, scopes: SCOPES, forceRefresh })
    .finally(() => {
      silentInFlight.delete(key);
    });
  silentInFlight.set(key, promise);
  return promise;
}

/** Vérifie l'accès OneDrive via l'API (évite les 401 Graph visibles dans la console). */
async function verifyOneDriveAccessToken(accessToken: string): Promise<boolean> {
  try {
    const res = await fetch("/api/onedrive/verify", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ accessToken }),
    });
    if (!res.ok) return false;
    const data = (await res.json()) as { ok?: boolean };
    return data.ok === true;
  } catch {
    return false;
  }
}

export function pickCachedAccessToken(cached: string | null | undefined): string | null {
  if (!cached?.trim() || isAccessTokenExpired(cached)) return null;
  return cached;
}

function isSilentIframeFailure(err: unknown): boolean {
  if (!(err instanceof msal.BrowserAuthError)) return false;
  const code = err.errorCode;
  return (
    code === "monitor_window_timeout" ||
    code === "iframe_closed_prematurely" ||
    code === "timed_out" ||
    code === "block_iframe_reload" ||
    code.includes("iframe")
  );
}

/** Restaure la session au chargement — sans redirection Microsoft. */
export async function tryRestoreOneDriveAccessToken(
  pca: msal.PublicClientApplication,
  account: msal.AccountInfo,
): Promise<string | null> {
  try {
    let tokenResponse = await acquireTokenSilentOnce(pca, account, false);
    if (
      !isAccessTokenExpired(tokenResponse.accessToken) &&
      (await verifyOneDriveAccessToken(tokenResponse.accessToken))
    ) {
      return tokenResponse.accessToken;
    }

    tokenResponse = await acquireTokenSilentOnce(pca, account, true);
    if (await verifyOneDriveAccessToken(tokenResponse.accessToken)) {
      return tokenResponse.accessToken;
    }
  } catch (err) {
    if (
      err instanceof msal.InteractionRequiredAuthError ||
      isSilentIframeFailure(err)
    ) {
      return null;
    }
    /* autre erreur réseau / cache — l'utilisateur pourra recliquer Se connecter */
  }
  return null;
}

/**
 * Token Graph valide pour OneDrive — silent, forceRefresh, puis redirect consent si besoin.
 * `acquireTokenRedirect` ne revient pas : la page se recharge après Microsoft.
 */
export async function obtainValidOneDriveAccessToken(
  pca: msal.PublicClientApplication,
  account: msal.AccountInfo,
): Promise<string> {
  try {
    let tokenResponse = await acquireTokenSilentOnce(pca, account, false);
    if (
      !isAccessTokenExpired(tokenResponse.accessToken) &&
      (await verifyOneDriveAccessToken(tokenResponse.accessToken))
    ) {
      return tokenResponse.accessToken;
    }

    tokenResponse = await acquireTokenSilentOnce(pca, account, true);
    if (await verifyOneDriveAccessToken(tokenResponse.accessToken)) {
      return tokenResponse.accessToken;
    }
  } catch (err) {
    if (
      !(err instanceof msal.InteractionRequiredAuthError) &&
      !isSilentIframeFailure(err)
    ) {
      throw err;
    }
  }

  storeMsalReturnPath();
  await pca.acquireTokenRedirect({
    account,
    scopes: SCOPES,
    prompt: "consent",
  });
  throw new Error("Redirection Microsoft en cours…");
}
