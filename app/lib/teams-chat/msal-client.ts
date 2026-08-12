"use client";

import * as msal from "@azure/msal-browser";
import { buildOneDriveMsalConfig } from "@/app/lib/msal-onedrive-client";

/** Mêmes identifiants que OneDrive (client ID + tenant ID). Pas de client secret. */
export const TEAMS_CHAT_MSAL_SCOPES = [
  "Chat.ReadWrite",
  "Chat.Create",
  "ChatMessage.Send",
  "User.Read",
  "People.Read",
  "offline_access",
] as const;

let pca: msal.PublicClientApplication | null = null;

async function fetchMicrosoftAppIds(): Promise<{ clientId: string; tenantId: string }> {
  const res = await fetch("/api/tenant/public");
  const tenant = (await res.json()) as {
    microsoftOneDrive?: { clientId?: string | null; tenantId?: string | null };
  };
  const clientId = String(tenant.microsoftOneDrive?.clientId || "").trim();
  const tenantId = String(tenant.microsoftOneDrive?.tenantId || "").trim();
  if (!clientId || !tenantId) {
    throw new Error("Microsoft n’est pas configuré pour cet établissement (client ID / tenant ID).");
  }
  return { clientId, tenantId };
}

export async function getTeamsChatMsal(): Promise<msal.PublicClientApplication> {
  if (pca) return pca;
  const ms = await fetchMicrosoftAppIds();
  pca = new msal.PublicClientApplication(buildOneDriveMsalConfig(ms));
  await pca.initialize();
  await pca.handleRedirectPromise();
  return pca;
}

export async function acquireTeamsChatToken(interactive: boolean): Promise<{
  accessToken: string;
  accountLabel: string;
} | null> {
  const instance = await getTeamsChatMsal();
  const scopes = [...TEAMS_CHAT_MSAL_SCOPES];
  const accounts = instance.getAllAccounts();

  if (accounts.length > 0) {
    try {
      const silent = await instance.acquireTokenSilent({ account: accounts[0], scopes });
      if (silent.accessToken) {
        return {
          accessToken: silent.accessToken,
          accountLabel: accounts[0].name || accounts[0].username || "Compte Microsoft",
        };
      }
    } catch {
      if (!interactive) return null;
    }
  }

  if (!interactive) return null;

  const result = await instance.loginPopup({
    scopes,
    prompt: accounts.length ? "consent" : "select_account",
  });
  if (!result.accessToken) return null;
  return {
    accessToken: result.accessToken,
    accountLabel: result.account?.name || result.account?.username || "Compte Microsoft",
  };
}
