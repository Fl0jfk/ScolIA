import "server-only";

import { getTenant, getTenantAppUrl } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import { RH_GRAPH_SCOPE_URL } from "@/app/lib/rh/graph-rh-drive";
import { formatRhOAuthError } from "@/app/lib/rh/oauth-errors";
import {
  buildMicrosoftAuthorizeUrl,
  microsoftAuthorizationCodeParams,
  postMicrosoftOAuthToken,
} from "@/app/lib/microsoft-oauth-token";

/** Cookie anti-CSRF OAuth OneDrive RH. */
export const RH_OAUTH_STATE_COOKIE = "rh_onedrive_oauth_state";

/**
 * Callback OAuth OneDrive RH.
 * RAPPEL AZURE : enregistrer cette URI en redirect Web sur l'App Registration :
 *   {appUrl}/api/rh/drive/oauth/callback
 * Messagerie Teams (même app) :
 *   {appUrl}/api/teams-chat/oauth/callback
 */
const RH_OAUTH_CALLBACK_PATH = "/api/rh/drive/oauth/callback";

export async function getRhOAuthRedirectUri(): Promise<string> {
  const base = await getTenantAppUrl();
  return `${base}${RH_OAUTH_CALLBACK_PATH}`;
}

async function rhOAuthClient(): Promise<{ clientId: string; tenantId: string }> {
  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const ms = secrets?.microsoft;
  const clientId = ms?.clientId || process.env.NEXT_PUBLIC_CLIENT_ID?.trim() || "";
  const tenantId = ms?.tenantId || process.env.NEXT_PUBLIC_TENANT_ID?.trim() || "";
  if (!clientId || !tenantId) {
    throw new Error("Microsoft non configuré (clientId / tenantId).");
  }
  return { clientId, tenantId };
}

export async function buildRhOAuthAuthorizeUrl(state: string): Promise<string> {
  const { clientId, tenantId } = await rhOAuthClient();
  return buildMicrosoftAuthorizeUrl({
    tenantId,
    clientId,
    redirectUri: await getRhOAuthRedirectUri(),
    scope: RH_GRAPH_SCOPE_URL,
    state,
  });
}

export async function exchangeRhOAuthCode(code: string): Promise<{
  accessToken: string;
  refreshToken: string;
}> {
  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const ms = secrets?.microsoft;
  const { clientId, tenantId } = await rhOAuthClient();
  if (!ms?.clientSecret?.trim()) {
    throw new Error(
      "clientSecret Microsoft requis pour lier le OneDrive RH (flux code serveur).",
    );
  }

  const redirectUri = await getRhOAuthRedirectUri();
  const result = await postMicrosoftOAuthToken(
    tenantId,
    microsoftAuthorizationCodeParams({
      clientId,
      clientSecret: ms.clientSecret.trim(),
      code,
      redirectUri,
      scope: RH_GRAPH_SCOPE_URL,
    }),
  );

  if (!result.ok) {
    throw new Error(formatRhOAuthError(`Échange code OAuth RH : ${result.body}`, redirectUri));
  }
  if (!result.tokens.refreshToken) {
    throw new Error(
      "Réponse OAuth sans refresh_token — vérifiez le scope offline_access et le type d'app (Web).",
    );
  }
  return { accessToken: result.tokens.accessToken, refreshToken: result.tokens.refreshToken };
}
