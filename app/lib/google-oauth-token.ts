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

const OAUTH_FETCH_TIMEOUT_MS = 20_000;
/** Même politique que `gcalFetch` (Calendar) : 3 essais + backoff. */
const OAUTH_FETCH_MAX_ATTEMPTS = 3;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
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

function isTransientOAuthNetworkError(err: unknown): boolean {
  const blob = describeOAuthNetworkError(err);
  return /fetch failed|ECONNRESET|ETIMEDOUT|ENOTFOUND|EAI_AGAIN|UND_ERR|socket|network|aborted|timeout/i.test(
    blob,
  );
}

function oauthNetworkFailureBody(err: unknown): string {
  const detail = describeOAuthNetworkError(err);
  return isTransientOAuthNetworkError(err)
    ? `Impossible de joindre Google OAuth (réseau) : ${detail}`
    : detail;
}

export async function postGoogleOAuthToken(
  params: URLSearchParams,
): Promise<{ ok: true; tokens: GoogleOAuthTokens } | { ok: false; status: number; body: string }> {
  let lastErr: unknown;
  for (let attempt = 1; attempt <= OAUTH_FETCH_MAX_ATTEMPTS; attempt += 1) {
    let res: Response;
    try {
      res = await fetch("https://oauth2.googleapis.com/token", {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: params,
        signal: AbortSignal.timeout(OAUTH_FETCH_TIMEOUT_MS),
      });
    } catch (e) {
      lastErr = e;
      if (!isTransientOAuthNetworkError(e) || attempt === OAUTH_FETCH_MAX_ATTEMPTS) {
        return { ok: false, status: 502, body: oauthNetworkFailureBody(e) };
      }
      await sleep(300 * 2 ** (attempt - 1));
      continue;
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

  return { ok: false, status: 502, body: oauthNetworkFailureBody(lastErr) };
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
