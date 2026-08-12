import "server-only";

import { getTenant, getTenantAppUrl } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import { RH_GRAPH_SCOPE_URL } from "@/app/lib/rh/graph-rh-drive";
import { formatRhOAuthError } from "@/app/lib/rh/oauth-errors";

/** Cookie anti-CSRF OAuth OneDrive RH. */
export const RH_OAUTH_STATE_COOKIE = "rh_onedrive_oauth_state";

/**
 * Callback OAuth OneDrive RH.
 * RAPPEL AZURE : enregistrer cette URI en redirect Web sur l'App Registration :
 *   {appUrl}/api/rh/drive/oauth/callback
 * Messagerie Teams (même app) :
 *   {appUrl}/api/teams-chat/oauth/callback
 */
export const RH_OAUTH_CALLBACK_PATH = "/api/rh/drive/oauth/callback";

export async function getRhOAuthRedirectUri(): Promise<string> {
  const base = await getTenantAppUrl();
  return `${base}${RH_OAUTH_CALLBACK_PATH}`;
}

export async function buildRhOAuthAuthorizeUrl(state: string): Promise<string> {
  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const ms = secrets?.microsoft;
  const clientId = ms?.clientId || process.env.NEXT_PUBLIC_CLIENT_ID?.trim() || "";
  const tenantId = ms?.tenantId || process.env.NEXT_PUBLIC_TENANT_ID?.trim() || "";
  if (!clientId || !tenantId) {
    throw new Error("Microsoft non configuré (clientId / tenantId).");
  }

  const redirectUri = await getRhOAuthRedirectUri();
  const params = new URLSearchParams({
    client_id: clientId,
    response_type: "code",
    redirect_uri: redirectUri,
    response_mode: "query",
    scope: RH_GRAPH_SCOPE_URL,
    state,
    prompt: "select_account",
  });

  return `https://login.microsoftonline.com/${tenantId}/oauth2/v2.0/authorize?${params.toString()}`;
}

export async function exchangeRhOAuthCode(code: string): Promise<{
  accessToken: string;
  refreshToken: string;
}> {
  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const ms = secrets?.microsoft;
  const clientId = ms?.clientId || process.env.NEXT_PUBLIC_CLIENT_ID?.trim() || "";
  const tenantId = ms?.tenantId || process.env.NEXT_PUBLIC_TENANT_ID?.trim() || "";
  if (!clientId || !tenantId) {
    throw new Error("Microsoft non configuré (clientId / tenantId).");
  }
  if (!ms?.clientSecret?.trim()) {
    throw new Error(
      "clientSecret Microsoft requis pour lier le OneDrive RH (flux code serveur).",
    );
  }

  const redirectUri = await getRhOAuthRedirectUri();
  const params = new URLSearchParams({
    client_id: clientId,
    client_secret: ms.clientSecret.trim(),
    code,
    redirect_uri: redirectUri,
    grant_type: "authorization_code",
    scope: RH_GRAPH_SCOPE_URL,
  });

  const res = await fetch(
    `https://login.microsoftonline.com/${tenantId}/oauth2/v2.0/token`,
    {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: params,
    },
  );

  if (!res.ok) {
    const err = await res.text();
    const redirectUri = await getRhOAuthRedirectUri();
    throw new Error(formatRhOAuthError(`Échange code OAuth RH : ${err}`, redirectUri));
  }

  const data = (await res.json()) as {
    access_token?: string;
    refresh_token?: string;
  };
  if (!data.access_token) throw new Error("Réponse OAuth sans access_token.");
  if (!data.refresh_token) {
    throw new Error(
      "Réponse OAuth sans refresh_token — vérifiez le scope offline_access et le type d'app (Web).",
    );
  }
  return { accessToken: data.access_token, refreshToken: data.refresh_token };
}
