"use client";

import * as msal from "@azure/msal-browser";
import { isAccessTokenExpired } from "@/app/lib/onedrive-access-token";
import { ONEDRIVE_MSAL_SCOPES, storeMsalReturnPath } from "@/app/lib/msal-onedrive-client";

const SCOPES = [...ONEDRIVE_MSAL_SCOPES];

/** Vérifie l'accès OneDrive via l'API (évite les 401 Graph visibles dans la console). */
async function verifyOneDriveAccessToken(accessToken: string): Promise<boolean> {
  try {
    const res = await fetch("/api/agentIAOCR/onedrive-verify", {
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

/** Restaure la session au chargement — sans redirection Microsoft. */
export async function tryRestoreOneDriveAccessToken(
  pca: msal.PublicClientApplication,
  account: msal.AccountInfo,
): Promise<string | null> {
  const trySilent = (forceRefresh: boolean) =>
    pca.acquireTokenSilent({ account, scopes: SCOPES, forceRefresh });

  try {
    let tokenResponse = await trySilent(false);
    if (
      !isAccessTokenExpired(tokenResponse.accessToken) &&
      (await verifyOneDriveAccessToken(tokenResponse.accessToken))
    ) {
      return tokenResponse.accessToken;
    }

    tokenResponse = await trySilent(true);
    if (await verifyOneDriveAccessToken(tokenResponse.accessToken)) {
      return tokenResponse.accessToken;
    }
  } catch {
    /* interaction requise — l'utilisateur devra cliquer sur Se connecter */
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
  const trySilent = (forceRefresh: boolean) =>
    pca.acquireTokenSilent({ account, scopes: SCOPES, forceRefresh });

  try {
    let tokenResponse = await trySilent(false);
    if (
      !isAccessTokenExpired(tokenResponse.accessToken) &&
      (await verifyOneDriveAccessToken(tokenResponse.accessToken))
    ) {
      return tokenResponse.accessToken;
    }

    tokenResponse = await trySilent(true);
    if (await verifyOneDriveAccessToken(tokenResponse.accessToken)) {
      return tokenResponse.accessToken;
    }
  } catch (err) {
    if (!(err instanceof msal.InteractionRequiredAuthError)) throw err;
  }

  storeMsalReturnPath();
  await pca.acquireTokenRedirect({
    account,
    scopes: SCOPES,
    prompt: "consent",
  });
  throw new Error("Redirection Microsoft en cours…");
}
