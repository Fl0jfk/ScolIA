import "server-only";

import { getTenant, getTenantAppUrl } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import {
  buildGoogleAuthorizeUrl,
  getGoogleAccessTokenFromRefresh,
  googleAuthorizationCodeParams,
  postGoogleOAuthToken,
} from "@/app/lib/google-oauth-token";

/** Cookie anti-CSRF OAuth Google Calendar (RDV inscriptions). */
export const RDV_INSCRIPTION_OAUTH_STATE_COOKIE = "rdv_inscription_oauth_state";

/**
 * Callback OAuth Google Calendar.
 * RAPPEL GOOGLE CLOUD : enregistrer cette URI en redirect Web :
 *   {appUrl}/api/rdv-inscription/oauth/callback
 */
const RDV_INSCRIPTION_OAUTH_CALLBACK_PATH = "/api/rdv-inscription/oauth/callback";

/** Scopes : lire / modifier les événements des agendas partagés avec le compte technique. */
export const RDV_INSCRIPTION_GOOGLE_SCOPES = [
  "https://www.googleapis.com/auth/calendar.events",
  "https://www.googleapis.com/auth/calendar.readonly",
  "https://www.googleapis.com/auth/userinfo.email",
  "openid",
].join(" ");

export async function getRdvInscriptionOAuthRedirectUri(): Promise<string> {
  const base = await getTenantAppUrl();
  return `${base}${RDV_INSCRIPTION_OAUTH_CALLBACK_PATH}`;
}

export async function resolveGoogleOAuthClient(): Promise<{
  clientId: string;
  clientSecret: string;
}> {
  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const g = secrets?.google;
  const clientId =
    g?.clientId?.trim() ||
    process.env.GOOGLE_CLIENT_ID?.trim() ||
    process.env.GOOGLE_CALENDAR_CLIENT_ID?.trim() ||
    "";
  const clientSecret =
    g?.clientSecret?.trim() ||
    process.env.GOOGLE_CLIENT_SECRET?.trim() ||
    process.env.GOOGLE_CALENDAR_CLIENT_SECRET?.trim() ||
    "";
  if (!clientId || !clientSecret) {
    throw new Error(
      "Google non configuré (GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET ou secrets.google).",
    );
  }
  return { clientId, clientSecret };
}

export async function buildRdvInscriptionOAuthAuthorizeUrl(state: string): Promise<string> {
  const { clientId } = await resolveGoogleOAuthClient();
  return buildGoogleAuthorizeUrl({
    clientId,
    redirectUri: await getRdvInscriptionOAuthRedirectUri(),
    scope: RDV_INSCRIPTION_GOOGLE_SCOPES,
    state,
    accessType: "offline",
    prompt: "consent",
  });
}

export async function exchangeRdvInscriptionOAuthCode(code: string): Promise<{
  accessToken: string;
  refreshToken: string;
}> {
  const { clientId, clientSecret } = await resolveGoogleOAuthClient();
  const redirectUri = await getRdvInscriptionOAuthRedirectUri();
  const result = await postGoogleOAuthToken(
    googleAuthorizationCodeParams({
      clientId,
      clientSecret,
      code,
      redirectUri,
    }),
  );
  if (!result.ok) {
    throw new Error(`Échange code OAuth Google : ${result.body || result.status}`);
  }
  if (!result.tokens.refreshToken) {
    throw new Error(
      "Réponse OAuth sans refresh_token — reconnectez avec prompt=consent (compte technique Google).",
    );
  }
  return {
    accessToken: result.tokens.accessToken,
    refreshToken: result.tokens.refreshToken,
  };
}

export async function fetchGoogleUserEmail(accessToken: string): Promise<{
  email: string | null;
  name: string | null;
}> {
  const res = await fetch("https://www.googleapis.com/oauth2/v2/userinfo", {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!res.ok) return { email: null, name: null };
  const data = (await res.json()) as { email?: string; name?: string };
  return {
    email: data.email?.trim() || null,
    name: data.name?.trim() || null,
  };
}

/** Access token prêt à l’emploi + rotation éventuelle du refresh token. */
export async function getRdvInscriptionGoogleAccessToken(): Promise<string> {
  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const refreshToken = secrets?.google?.calendar?.refreshToken?.trim();
  if (!refreshToken) {
    throw new Error("Compte Google Calendar non lié — connectez-le dans le paramétrage RDV.");
  }
  const { clientId, clientSecret } = await resolveGoogleOAuthClient();
  const tokens = await getGoogleAccessTokenFromRefresh({
    clientId,
    clientSecret,
    refreshToken,
  });
  if (tokens.refreshToken && tokens.refreshToken !== refreshToken) {
    const { persistRotatedGoogleRefreshToken } = await import(
      "@/app/lib/rdv-inscription-google"
    );
    await persistRotatedGoogleRefreshToken(tokens.refreshToken);
  }
  return tokens.accessToken;
}
