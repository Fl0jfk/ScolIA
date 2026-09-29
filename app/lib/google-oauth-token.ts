import "server-only";

/** Helpers HTTP OAuth Google (Calendar) — scopes passés par l’appelant. */

export type GoogleOAuthTokens = {
  accessToken: string;
  refreshToken?: string;
  expiresIn?: number;
};

export function buildGoogleAuthorizeUrl(opts: {
  clientId: string;
  redirectUri: string;
  scope: string;
  state: string;
  accessType?: "offline" | "online";
  prompt?: string;
}): string {
  const params = new URLSearchParams({
    client_id: opts.clientId,
    response_type: "code",
    redirect_uri: opts.redirectUri,
    scope: opts.scope,
    state: opts.state,
    access_type: opts.accessType ?? "offline",
    include_granted_scopes: "true",
    prompt: opts.prompt ?? "consent",
  });
  return `https://accounts.google.com/o/oauth2/v2/auth?${params.toString()}`;
}

function describeOAuthNetworkError(err: unknown): string {
  const parts: string[] = [];
  let cur: unknown = err;
  for (let depth = 0; depth < 4 && cur; depth += 1) {
    if (cur instanceof Error) {
      const code =
        "code" in cur && typeof (cur as { code?: unknown }).code === "string"
          ? (cur as { code: string }).code
          : null;
      parts.push(code ? `${cur.message} (${code})` : cur.message);
      cur = cur.cause;
      continue;
    }
    parts.push(String(cur));
    break;
  }
  return parts.filter(Boolean).join(" ← ") || "erreur réseau";
}

export async function postGoogleOAuthToken(
  params: URLSearchParams,
): Promise<{ ok: true; tokens: GoogleOAuthTokens } | { ok: false; status: number; body: string }> {
  let res: Response;
  try {
    res = await fetch("https://oauth2.googleapis.com/token", {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: params,
      signal: AbortSignal.timeout(20_000),
    });
  } catch (e) {
    const detail = describeOAuthNetworkError(e);
    return {
      ok: false,
      status: 502,
      body: /fetch failed|ECONNRESET|ETIMEDOUT|ENOTFOUND|EAI_AGAIN|UND_ERR|network|timeout/i.test(
        detail,
      )
        ? `Impossible de joindre Google OAuth (réseau) : ${detail}`
        : detail,
    };
  }
  const body = await res.text();
  if (!res.ok) return { ok: false, status: res.status, body };
  let parsed: {
    access_token?: string;
    refresh_token?: string;
    expires_in?: number;
  };
  try {
    parsed = JSON.parse(body) as {
      access_token?: string;
      refresh_token?: string;
      expires_in?: number;
    };
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
      expiresIn: typeof parsed.expires_in === "number" ? parsed.expires_in : undefined,
    },
  };
}

export function googleAuthorizationCodeParams(opts: {
  clientId: string;
  clientSecret: string;
  code: string;
  redirectUri: string;
}): URLSearchParams {
  return new URLSearchParams({
    client_id: opts.clientId,
    client_secret: opts.clientSecret,
    code: opts.code,
    redirect_uri: opts.redirectUri,
    grant_type: "authorization_code",
  });
}

export function googleRefreshTokenParams(opts: {
  clientId: string;
  clientSecret: string;
  refreshToken: string;
}): URLSearchParams {
  return new URLSearchParams({
    client_id: opts.clientId,
    client_secret: opts.clientSecret,
    refresh_token: opts.refreshToken,
    grant_type: "refresh_token",
  });
}

export async function getGoogleAccessTokenFromRefresh(opts: {
  clientId: string;
  clientSecret: string;
  refreshToken: string;
}): Promise<GoogleOAuthTokens> {
  const result = await postGoogleOAuthToken(
    googleRefreshTokenParams({
      clientId: opts.clientId,
      clientSecret: opts.clientSecret,
      refreshToken: opts.refreshToken,
    }),
  );
  if (!result.ok) {
    throw new Error(`Refresh token Google : ${result.body || result.status}`);
  }
  return result.tokens;
}
