"use client";

import type { Configuration } from "@azure/msal-browser";

export const ONEDRIVE_MSAL_SCOPES = ["Files.ReadWrite", "User.Read", "offline_access"] as const;

/** Page légère — la popup MSAL ne doit pas charger toute l'app OCR. */
const ONEDRIVE_MSAL_CALLBACK_PATH = "/agentIAOCR/msal-callback";

type MicrosoftOneDrivePublicConfig = {
  enabled: boolean;
  clientId: string;
  tenantId: string;
};

function oneDriveMsalRedirectUri(): string {
  if (typeof window === "undefined") return ONEDRIVE_MSAL_CALLBACK_PATH;
  return `${window.location.origin}${ONEDRIVE_MSAL_CALLBACK_PATH}`;
}

const MSAL_RETURN_PATH_KEY = "onedrive_msal_return_path";

export function storeMsalReturnPath(path?: string): void {
  if (typeof window === "undefined") return;
  const next = path ?? `${window.location.pathname}${window.location.search}`;
  sessionStorage.setItem(MSAL_RETURN_PATH_KEY, next);
}

export function consumeMsalReturnPath(): string {
  if (typeof window === "undefined") return "/agentIAOCR";
  const stored = sessionStorage.getItem(MSAL_RETURN_PATH_KEY);
  sessionStorage.removeItem(MSAL_RETURN_PATH_KEY);
  if (stored && stored.startsWith("/")) return stored;
  return "/agentIAOCR";
}

export async function fetchMicrosoftOneDrivePublicConfig(): Promise<MicrosoftOneDrivePublicConfig | null> {
  const tenantRes = await fetch("/api/tenant/public");
  const tenant = await tenantRes.json();
  const ms = tenant.microsoftOneDrive;
  if (!ms?.enabled || !ms.clientId || !ms.tenantId) return null;
  return {
    enabled: true,
    clientId: String(ms.clientId),
    tenantId: String(ms.tenantId),
  };
}

export function buildOneDriveMsalConfig(ms: Pick<MicrosoftOneDrivePublicConfig, "clientId" | "tenantId">): Configuration {
  return {
    auth: {
      clientId: ms.clientId,
      authority: `https://login.microsoftonline.com/${ms.tenantId}`,
      redirectUri: oneDriveMsalRedirectUri(),
    },
    cache: {
      cacheLocation: "localStorage",
    },
  };
}
