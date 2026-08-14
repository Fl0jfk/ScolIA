/** Décodage léger du JWT Microsoft (claim `exp` uniquement). */
function getAccessTokenExpiresAt(token: string): number | null {
  try {
    const parts = token.split(".");
    if (parts.length < 2) return null;
    const payload = JSON.parse(atob(parts[1].replace(/-/g, "+").replace(/_/g, "/"))) as { exp?: unknown };
    return typeof payload.exp === "number" ? payload.exp : null;
  } catch {
    return null;
  }
}

export function isAccessTokenExpired(token: string, skewSeconds = 90): boolean {
  const exp = getAccessTokenExpiresAt(token);
  if (!exp) return true;
  return Date.now() >= exp * 1000 - skewSeconds * 1000;
}
