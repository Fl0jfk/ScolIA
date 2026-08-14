import "server-only";

/** Helpers HTTP OAuth Microsoft (scopes passés par l’appelant, inchangés). */

export function buildMicrosoftAuthorizeUrl(opts: {
  tenantId: string;
  clientId: string;
  redirectUri: string;
  scope: string;
  state: string;
  prompt?: string;
}): string {
  const params = new URLSearchParams({
    client_id: opts.clientId,
    response_type: "code",
    redirect_uri: opts.redirectUri,
    response_mode: "query",
    scope: opts.scope,
    state: opts.state,
    prompt: opts.prompt ?? "select_account",
  });
  return `https://login.microsoftonline.com/${opts.tenantId}/oauth2/v2.0/authorize?${params.toString()}`;
}

type MicrosoftOAuthTokens = {
  accessToken: string;
  refreshToken?: string;
};

export async function postMicrosoftOAuthToken(
  tenantId: string,
  params: URLSearchParams,
): Promise<{ ok: true; tokens: MicrosoftOAuthTokens } | { ok: false; status: number; body: string }> {
  const res = await fetch(
    `https://login.microsoftonline.com/${tenantId}/oauth2/v2.0/token`,
    {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: params,
    },
  );
  const body = await res.text();
  if (!res.ok) return { ok: false, status: res.status, body };
  let parsed: { access_token?: string; refresh_token?: string };
  try {
    parsed = JSON.parse(body) as { access_token?: string; refresh_token?: string };
  } catch {
    return { ok: false, status: res.status, body };
  }
  if (!parsed.access_token) {
    return { ok: false, status: res.status, body: body || "Réponse sans access_token." };
  }
  return {
    ok: true,
    tokens: {
      accessToken: parsed.access_token,
      refreshToken: parsed.refresh_token?.trim() || undefined,
    },
  };
}

export function microsoftAuthorizationCodeParams(opts: {
  clientId: string;
  clientSecret: string;
  code: string;
  redirectUri: string;
  scope: string;
}): URLSearchParams {
  return new URLSearchParams({
    client_id: opts.clientId,
    client_secret: opts.clientSecret,
    code: opts.code,
    redirect_uri: opts.redirectUri,
    grant_type: "authorization_code",
    scope: opts.scope,
  });
}

export function microsoftRefreshTokenParams(opts: {
  clientId: string;
  refreshToken: string;
  scope: string;
  clientSecret?: string;
}): URLSearchParams {
  const params = new URLSearchParams({
    client_id: opts.clientId,
    refresh_token: opts.refreshToken.trim(),
    grant_type: "refresh_token",
    scope: opts.scope,
  });
  if (opts.clientSecret) params.set("client_secret", opts.clientSecret);
  return params;
}
