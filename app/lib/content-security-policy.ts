import { SCOLA_IMAGE_CDN_HOST } from "./scola-image";

const isDev = process.env.NODE_ENV !== "production";

function compactCsp(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function contentSecurityPolicyDirectives(nonce?: string): string {
  const noncePart = nonce ? ` 'nonce-${nonce}'` : "";
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
    https://clerk.scolia.fr
    https://accounts.scolia.fr
    https://clerk.lpnb.scolia.fr
    https://accounts.lpnb.scolia.fr
    https://clerk.docslapro.com
    https://accounts.docslapro.com
    https://clerk.lp.docslapro.com
    https://accounts.lp.docslapro.com;
  object-src 'self' blob: data:;
  connect-src 'self'
    https://*.s3.fr-par.scw.cloud
    https://s3.fr-par.scw.cloud
    https://${SCOLA_IMAGE_CDN_HOST}
    https://clerk-telemetry.com
    https://*.clerk-telemetry.com
    https://api.stripe.com
    https://maps.googleapis.com
    https://clerk.scolia.fr
    https://accounts.scolia.fr
    https://clerk.lpnb.scolia.fr
    https://accounts.lpnb.scolia.fr
    https://clerk.docslapro.com
    https://accounts.docslapro.com
    https://clerk.lp.docslapro.com
    https://accounts.lp.docslapro.com
    https://www.googleapis.com
    genuine-wildcat-70.clerk.accounts.dev
    https://login.microsoftonline.com
    https://graph.microsoft.com;
  worker-src 'self' blob:;
  form-action 'self' https://*.s3.fr-par.scw.cloud https://s3.fr-par.scw.cloud;
  img-src 'self' https://img.clerk.com https://clerk.scolia.fr https://clerk.lpnb.scolia.fr https://clerk.docslapro.com https://clerk.lp.docslapro.com https: data:;
  script-src 'self' 'unsafe-inline'${noncePart}${isDev ? " 'unsafe-eval'" : ""} https:;
  style-src 'self' 'unsafe-inline'${noncePart} https:;
  font-src 'self' https: data:;
`;
}

/** CSP enforce : `unsafe-inline` conservé ; wildcard `https:` scripts/styles conservé. Nonce optionnel (pipeline Next/Clerk). */
export function contentSecurityPolicyHeaderValue(nonce?: string): string {
  return compactCsp(contentSecurityPolicyDirectives(nonce));
}

/** Popups OAuth Microsoft (MSAL loginPopup / acquireTokenSilent iframe). */
export function crossOriginOpenerPolicyHeaderValue(): string {
  return "same-origin-allow-popups";
}
