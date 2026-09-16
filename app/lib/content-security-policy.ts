import { SCOLA_IMAGE_CDN_HOST } from "./scola-image";

const isDev = process.env.NODE_ENV !== "production";

function compactCsp(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

/** Origines Collabora CODE autorisées en iframe (frame-src). */
function collaboraFrameSrcOrigins(): string[] {
  const origins = new Set<string>([
    "https://office.scolia.fr",
    "https://*.functions.fnc.fr-par.scw.cloud",
  ]);
  if (isDev) {
    origins.add("http://127.0.0.1:9980");
    origins.add("http://localhost:9980");
  }
  for (const raw of [
    process.env.COLLABORA_URL,
    process.env.NEXT_PUBLIC_COLLABORA_URL,
  ]) {
    const u = raw?.trim();
    if (!u) continue;
    try {
      origins.add(new URL(u).origin);
    } catch {
      /* ignore */
    }
  }
  return [...origins];
}

function contentSecurityPolicyDirectives(nonce?: string): string {
  const scriptSrc = nonce
    ? `'self' 'nonce-${nonce}' 'strict-dynamic'${isDev ? " 'unsafe-eval'" : ""}`
    : `'self'${isDev ? " 'unsafe-eval'" : ""}`;
  const collaboraFrames = collaboraFrameSrcOrigins().join("\n    ");
  return `
  default-src 'self' https://login.microsoftonline.com/;
  base-uri 'self';
  frame-ancestors 'self';
  frame-src 'self' blob:
    https://www.google.com/maps/
    https://maps.google.com/
    https://login.microsoftonline.com
    https://*.microsoftonline.com
    https://login.live.com
    https://login.microsoft.com
    ${collaboraFrames};
  object-src 'self' blob: data:;
  connect-src 'self'
    https://*.s3.fr-par.scw.cloud
    https://s3.fr-par.scw.cloud
    https://${SCOLA_IMAGE_CDN_HOST}
    https://api.stripe.com
    https://maps.googleapis.com
    https://www.googleapis.com
    https://login.microsoftonline.com
    https://graph.microsoft.com
    ${collaboraFrames};
  worker-src 'self' blob:;
  form-action 'self' https://*.s3.fr-par.scw.cloud https://s3.fr-par.scw.cloud;
  img-src 'self' https: data:;
  script-src ${scriptSrc};
  style-src 'self' 'unsafe-inline';
  font-src 'self' https: data:;
`;
}

/**
 * CSP enforce : nonce + strict-dynamic sur les scripts.
 * style-src : pas de nonce — en CSP3, un nonce dans style-src annule 'unsafe-inline',
 * ce qui bloque les <style> de Next et Framer Motion (dashboard).
 */
export function contentSecurityPolicyHeaderValue(nonce?: string): string {
  return compactCsp(contentSecurityPolicyDirectives(nonce));
}

/** Popups OAuth Microsoft (MSAL loginPopup / acquireTokenSilent iframe). */
export function crossOriginOpenerPolicyHeaderValue(): string {
  return "same-origin-allow-popups";
}
