import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";
import { normalizeHostname } from "@/app/lib/tenant-registry";

/** Anciens hôtes DocsLaPro → origine ScolIA (même chemin). */
const LEGACY_HOST_TO_ORIGIN: Record<string, string> = {
  "lp.docslapro.com": "https://lpnb.scolia.fr",
  "www.lp.docslapro.com": "https://lpnb.scolia.fr",
};

export function legacyDocsLaProRedirect(request: NextRequest): NextResponse | null {
  const host = normalizeHostname(
    request.headers.get("x-forwarded-host") ||
      request.headers.get("host") ||
      request.nextUrl.hostname,
  );
  const origin = LEGACY_HOST_TO_ORIGIN[host];
  if (!origin) return null;

  const dest = new URL(`${request.nextUrl.pathname}${request.nextUrl.search}`, origin);
  return NextResponse.redirect(dest, 308);
}
