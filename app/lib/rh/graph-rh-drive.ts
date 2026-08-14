import "server-only";

import { loadAppConfig, saveIntegrations } from "@/app/lib/app-config";
import { getMicrosoftAccessTokenFromRefresh } from "@/app/lib/graph-microsoft-delegated";
import { RH_DEFAULT_BASE_PATH, normalizeRhBasePath } from "@/app/lib/rh/paths";
import { getRhOAuthRedirectUri } from "@/app/lib/rh/oauth-rh-drive";
import { getTenant } from "@/app/lib/tenant-context";
import {
  getTenantSecrets,
  loadTenantSecretsFile,
  saveTenantSecretsFile,
} from "@/app/lib/tenant-registry";
import type { TenantSecrets } from "@/app/lib/tenant-types";

const RH_GRAPH_SCOPES = "Files.ReadWrite offline_access User.Read";
export const RH_GRAPH_SCOPE_URL =
  "https://graph.microsoft.com/Files.ReadWrite offline_access User.Read";

type RhDriveStatus = {
  enabled: boolean;
  linked: boolean;
  healthy: boolean;
  linkedUpn: string | null;
  linkedDisplayName: string | null;
  linkedAt: string | null;
  basePath: string;
  /** URI exacte à copier dans Azure → Authentification → URI de redirection Web. */
  oauthRedirectUri: string | null;
  error: string | null;
};

function mergeMicrosoft(
  existing: TenantSecrets["microsoft"] | undefined,
  patch: Partial<NonNullable<TenantSecrets["microsoft"]>>,
): NonNullable<TenantSecrets["microsoft"]> {
  if (!existing?.tenantId || !existing.clientId) {
    throw new Error("Microsoft non configuré pour ce tenant (clientId / tenantId).");
  }
  return {
    ...existing,
    ...patch,
    tenantId: existing.tenantId,
    clientId: existing.clientId,
    clientSecret: patch.clientSecret ?? existing.clientSecret,
    oneDriveBySecteur: patch.oneDriveBySecteur ?? existing.oneDriveBySecteur,
    rhDrive: patch.rhDrive !== undefined ? patch.rhDrive : existing.rhDrive,
  };
}

export async function getRhDrivePublicConfig() {
  const config = await loadAppConfig();
  const rh = config.integrations.microsoftOneDrive?.rhDrive;
  return {
    enabled: rh?.enabled !== false,
    linked: rh?.linked === true,
    linkedUpn: rh?.linkedUpn ?? null,
    linkedDisplayName: rh?.linkedDisplayName ?? null,
    linkedAt: rh?.linkedAt ?? null,
    basePath: normalizeRhBasePath(rh?.basePath),
  };
}

export async function saveRhDriveLinkSecret(input: {
  refreshToken: string;
  linkedUpn?: string;
  linkedDisplayName?: string;
}): Promise<void> {
  const tenant = await getTenant();
  const existing = (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  if (!existing) throw new Error("Secrets tenant introuvables.");

  const linkedAt = new Date().toISOString();
  const microsoft = mergeMicrosoft(existing.microsoft, {
    rhDrive: {
      refreshToken: input.refreshToken.trim(),
      linkedUpn: input.linkedUpn?.trim() || undefined,
      linkedDisplayName: input.linkedDisplayName?.trim() || undefined,
      linkedAt,
    },
  });

  await saveTenantSecretsFile(tenant.slug, { ...existing, microsoft });

  const config = await loadAppConfig();
  const oneDrive = config.integrations.microsoftOneDrive ?? { enabled: true };
  await saveIntegrations({
    ...config.integrations,
    microsoftOneDrive: {
      ...oneDrive,
      enabled: oneDrive.enabled !== false,
      rhDrive: {
        enabled: true,
        linked: true,
        linkedUpn: input.linkedUpn?.trim() || undefined,
        linkedDisplayName: input.linkedDisplayName?.trim() || undefined,
        linkedAt,
        basePath: normalizeRhBasePath(oneDrive.rhDrive?.basePath || RH_DEFAULT_BASE_PATH),
      },
    },
  });
}

export async function clearRhDriveLink(): Promise<void> {
  const tenant = await getTenant();
  const existing = (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  if (!existing?.microsoft) throw new Error("Secrets Microsoft introuvables.");

  const { rhDrive: _removed, ...restMs } = existing.microsoft;
  await saveTenantSecretsFile(tenant.slug, {
    ...existing,
    microsoft: restMs,
  });

  const config = await loadAppConfig();
  const oneDrive = config.integrations.microsoftOneDrive ?? { enabled: true };
  await saveIntegrations({
    ...config.integrations,
    microsoftOneDrive: {
      ...oneDrive,
      enabled: oneDrive.enabled !== false,
      rhDrive: {
        enabled: true,
        linked: false,
        basePath: normalizeRhBasePath(oneDrive.rhDrive?.basePath || RH_DEFAULT_BASE_PATH),
      },
    },
  });
}

async function persistRotatedRefreshToken(newRefreshToken: string): Promise<void> {
  const tenant = await getTenant();
  const existing = (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  const current = existing?.microsoft?.rhDrive;
  if (!existing?.microsoft || !current?.refreshToken) return;

  await saveTenantSecretsFile(tenant.slug, {
    ...existing,
    microsoft: {
      ...existing.microsoft,
      rhDrive: {
        ...current,
        refreshToken: newRefreshToken,
      },
    },
  });
}

/**
 * Access token Graph pour le OneDrive RH (refresh délégué de l'attachée).
 * Abstraction prête pour un futur mode app-only.
 */
export async function getRhDriveAccessToken(): Promise<
  { accessToken: string; basePath: string } | { error: string; code: string }
> {
  const publicCfg = await getRhDrivePublicConfig();
  if (!publicCfg.enabled) {
    return { error: "OneDrive RH désactivé.", code: "RH_DRIVE_DISABLED" };
  }

  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const refreshToken = secrets?.microsoft?.rhDrive?.refreshToken?.trim();
  if (!refreshToken) {
    return {
      error: "OneDrive RH non connecté. Reliez le compte de l'attachée de gestion.",
      code: "RH_DRIVE_NOT_LINKED",
    };
  }

  const tokenResult = await getMicrosoftAccessTokenFromRefresh(refreshToken);
  if ("error" in tokenResult) {
    return { error: tokenResult.error, code: "RH_DRIVE_TOKEN_ERROR" };
  }

  if (tokenResult.refreshToken && tokenResult.refreshToken !== refreshToken) {
    try {
      await persistRotatedRefreshToken(tokenResult.refreshToken);
    } catch (e) {
      console.error("[rh-drive] rotation refresh token", e);
    }
  }

  return { accessToken: tokenResult.accessToken, basePath: publicCfg.basePath };
}

export async function probeRhDriveHealth(): Promise<RhDriveStatus> {
  let oauthRedirectUri: string | null = null;
  try {
    oauthRedirectUri = await getRhOAuthRedirectUri();
  } catch {
    oauthRedirectUri = null;
  }

  const publicCfg = await getRhDrivePublicConfig();
  const base: RhDriveStatus = {
    enabled: publicCfg.enabled,
    linked: publicCfg.linked,
    healthy: false,
    linkedUpn: publicCfg.linkedUpn,
    linkedDisplayName: publicCfg.linkedDisplayName,
    linkedAt: publicCfg.linkedAt,
    basePath: publicCfg.basePath,
    oauthRedirectUri,
    error: null,
  };

  if (!publicCfg.enabled) {
    return { ...base, error: "OneDrive RH désactivé." };
  }
  if (!publicCfg.linked) {
    return { ...base, error: "OneDrive RH non connecté." };
  }

  const token = await getRhDriveAccessToken();
  if ("error" in token) {
    return { ...base, healthy: false, error: token.error };
  }

  try {
    const res = await fetch("https://graph.microsoft.com/v1.0/me/drive", {
      headers: { Authorization: `Bearer ${token.accessToken}` },
    });
    if (!res.ok) {
      const err = await res.text();
      return {
        ...base,
        healthy: false,
        error: `Graph /me/drive : ${err.slice(0, 180)}`,
      };
    }
    const drive = (await res.json()) as {
      owner?: { user?: { displayName?: string; email?: string } };
    };
    return {
      ...base,
      healthy: true,
      linkedUpn: drive.owner?.user?.email ?? base.linkedUpn,
      linkedDisplayName: drive.owner?.user?.displayName ?? base.linkedDisplayName,
      error: null,
    };
  } catch (e) {
    return {
      ...base,
      healthy: false,
      error: e instanceof Error ? e.message : "Échec ping OneDrive RH",
    };
  }
}

export async function fetchRhGraphMe(accessToken: string): Promise<{
  displayName?: string;
  mail?: string;
  userPrincipalName?: string;
}> {
  const res = await fetch("https://graph.microsoft.com/v1.0/me?$select=displayName,mail,userPrincipalName", {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!res.ok) {
    throw new Error(`Graph /me : ${(await res.text()).slice(0, 180)}`);
  }
  return (await res.json()) as {
    displayName?: string;
    mail?: string;
    userPrincipalName?: string;
  };
}
