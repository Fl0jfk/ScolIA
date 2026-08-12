import "server-only";

import { getTenant, getTenantAppUrl } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";

/** Cookie anti-CSRF OAuth Teams (compte Microsoft personnel). */
export const TEAMS_CHAT_OAUTH_STATE_COOKIE = "teams_chat_oauth_state";

/**
 * Callback OAuth messagerie Teams.
 * RAPPEL AZURE : enregistrer cette URI en redirect Web sur l'App Registration Scola :
 *   {appUrl}/api/teams-chat/oauth/callback
 *
 * Scopes délégués :
 *   Chat.ReadWrite, Chat.Create, ChatMessage.Send, User.Read, People.Read, offline_access
 * People.Read suffit pour chercher des collègues (pas besoin de User.Read.All / consentement admin).
 */
export const TEAMS_CHAT_OAUTH_CALLBACK_PATH = "/api/teams-chat/oauth/callback";

export const TEAMS_CHAT_GRAPH_SCOPES = [
  "https://graph.microsoft.com/Chat.ReadWrite",
  "https://graph.microsoft.com/Chat.Create",
  "https://graph.microsoft.com/ChatMessage.Send",
  "https://graph.microsoft.com/User.Read",
  "https://graph.microsoft.com/People.Read",
  "offline_access",
].join(" ");

export async function getTeamsChatOAuthRedirectUri(): Promise<string> {
  const base = await getTenantAppUrl();
  return `${base}${TEAMS_CHAT_OAUTH_CALLBACK_PATH}`;
}

async function microsoftAppCreds(): Promise<{
  clientId: string;
  tenantId: string;
  clientSecret: string;
}> {
  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const ms = secrets?.microsoft;
  const clientId = ms?.clientId || process.env.NEXT_PUBLIC_CLIENT_ID?.trim() || "";
  const tenantId = ms?.tenantId || process.env.NEXT_PUBLIC_TENANT_ID?.trim() || "";
  const clientSecret = ms?.clientSecret?.trim() || "";
  if (!clientId || !tenantId) {
    throw new Error("Microsoft non configuré (clientId / tenantId).");
  }
  if (!clientSecret) {
    throw new Error("clientSecret Microsoft requis pour lier le compte Teams (flux code serveur).");
  }
  return { clientId, tenantId, clientSecret };
}

export async function buildTeamsChatOAuthAuthorizeUrl(state: string): Promise<string> {
  const { clientId, tenantId } = await microsoftAppCreds();
  const redirectUri = await getTeamsChatOAuthRedirectUri();
  const params = new URLSearchParams({
    client_id: clientId,
    response_type: "code",
    redirect_uri: redirectUri,
    response_mode: "query",
    scope: TEAMS_CHAT_GRAPH_SCOPES,
    state,
    prompt: "select_account",
  });
  return `https://login.microsoftonline.com/${tenantId}/oauth2/v2.0/authorize?${params.toString()}`;
}

export async function exchangeTeamsChatOAuthCode(code: string): Promise<{
  accessToken: string;
  refreshToken: string;
}> {
  const { clientId, tenantId, clientSecret } = await microsoftAppCreds();
  const redirectUri = await getTeamsChatOAuthRedirectUri();
  const params = new URLSearchParams({
    client_id: clientId,
    client_secret: clientSecret,
    code,
    redirect_uri: redirectUri,
    grant_type: "authorization_code",
    scope: TEAMS_CHAT_GRAPH_SCOPES,
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
    throw new Error(`Échange code OAuth Teams : ${err.slice(0, 400)}`);
  }

  const data = (await res.json()) as {
    access_token?: string;
    refresh_token?: string;
  };
  if (!data.access_token) throw new Error("Réponse OAuth Teams sans access_token.");
  if (!data.refresh_token) {
    throw new Error(
      "Réponse OAuth sans refresh_token — vérifiez offline_access et le type d'app (Web).",
    );
  }
  return { accessToken: data.access_token, refreshToken: data.refresh_token };
}

export async function refreshTeamsChatAccessToken(refreshToken: string): Promise<{
  accessToken: string;
  refreshToken?: string;
}> {
  const { clientId, tenantId, clientSecret } = await microsoftAppCreds();
  const params = new URLSearchParams({
    client_id: clientId,
    client_secret: clientSecret,
    refresh_token: refreshToken.trim(),
    grant_type: "refresh_token",
    scope: TEAMS_CHAT_GRAPH_SCOPES,
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
    throw new Error(`Refresh token Teams : ${err.slice(0, 400)}`);
  }

  const data = (await res.json()) as {
    access_token?: string;
    refresh_token?: string;
  };
  if (!data.access_token) throw new Error("Réponse refresh Teams sans access_token.");
  return {
    accessToken: data.access_token,
    refreshToken: data.refresh_token?.trim() || undefined,
  };
}
