import "server-only";

import { getTenant, getTenantAppUrl } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import {
  buildMicrosoftAuthorizeUrl,
  microsoftAuthorizationCodeParams,
  microsoftRefreshTokenParams,
  postMicrosoftOAuthToken,
} from "@/app/lib/microsoft-oauth-token";

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
const TEAMS_CHAT_OAUTH_CALLBACK_PATH = "/api/teams-chat/oauth/callback";

const TEAMS_CHAT_GRAPH_SCOPES = [
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
  const clientSecret =
    ms?.clientSecret?.trim() ||
    process.env.MICROSOFT_CLIENT_SECRET?.trim() ||
    process.env.CLIENT_SECRET?.trim() ||
    "";
  if (!clientId || !tenantId) {
    throw new Error("Microsoft non configuré (clientId / tenantId).");
  }
  if (!clientSecret) {
    throw new Error(
      "Secret client Microsoft manquant — ce n’est pas l’ID du tenant. Entra → Inscription d’application → Certificats et secrets → Nouvelle clé secrète, puis coller la Valeur dans Scola (Plateforme → Microsoft — client secret). La messagerie peut aussi se lier via MSAL (même flux que OneDrive), sans ce secret.",
    );
  }
  return { clientId, tenantId, clientSecret };
}

export async function buildTeamsChatOAuthAuthorizeUrl(state: string): Promise<string> {
  const { clientId, tenantId } = await microsoftAppCreds();
  return buildMicrosoftAuthorizeUrl({
    tenantId,
    clientId,
    redirectUri: await getTeamsChatOAuthRedirectUri(),
    scope: TEAMS_CHAT_GRAPH_SCOPES,
    state,
  });
}

export async function exchangeTeamsChatOAuthCode(code: string): Promise<{
  accessToken: string;
  refreshToken: string;
}> {
  const { clientId, tenantId, clientSecret } = await microsoftAppCreds();
  const redirectUri = await getTeamsChatOAuthRedirectUri();
  const result = await postMicrosoftOAuthToken(
    tenantId,
    microsoftAuthorizationCodeParams({
      clientId,
      clientSecret,
      code,
      redirectUri,
      scope: TEAMS_CHAT_GRAPH_SCOPES,
    }),
  );

  if (!result.ok) {
    throw new Error(`Échange code OAuth Teams : ${result.body.slice(0, 400)}`);
  }
  if (!result.tokens.refreshToken) {
    throw new Error(
      "Réponse OAuth sans refresh_token — vérifiez offline_access et le type d'app (Web).",
    );
  }
  return { accessToken: result.tokens.accessToken, refreshToken: result.tokens.refreshToken };
}

export async function refreshTeamsChatAccessToken(refreshToken: string): Promise<{
  accessToken: string;
  refreshToken?: string;
}> {
  const { clientId, tenantId, clientSecret } = await microsoftAppCreds();
  const result = await postMicrosoftOAuthToken(
    tenantId,
    microsoftRefreshTokenParams({
      clientId,
      clientSecret,
      refreshToken,
      scope: TEAMS_CHAT_GRAPH_SCOPES,
    }),
  );

  if (!result.ok) {
    throw new Error(`Refresh token Teams : ${result.body.slice(0, 400)}`);
  }
  return {
    accessToken: result.tokens.accessToken,
    refreshToken: result.tokens.refreshToken,
  };
}
