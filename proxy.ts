import { createClerkClient } from '@clerk/backend';
import {
  clerkMiddleware,
  createRouteMatcher,
  type ClerkMiddlewareAuth,
} from '@clerk/nextjs/server';
import { NextResponse } from 'next/server';
import type { NextRequest, NextFetchEvent } from 'next/server';
import { TENANT_SLUG_HEADER, TENANT_REQUEST_URL_HEADER, type TenantConfig } from '@/app/lib/tenant-types';
import {
  defaultTenantFromEnv,
  isMultiTenantEnabled,
  normalizeHostname,
  resolveLocalDevTenantBySlug,
  resolveLocalDevTenantFromList,
  resolveTenantByHostname,
  resolveTenantByHostnameSync,
  warmTenantRegistry,
  getCachedTenants,
} from '@/app/lib/tenant-registry';
import { isLocalDevHostname, resolveClerkKeysForHostname } from '@/app/lib/clerk-tenant-keys';
import {
  LOCAL_DEV_TENANT_COOKIE,
  LOCAL_DEV_TENANT_QUERY,
} from '@/app/lib/local-dev';
import { isPlatformHostname } from '@/app/lib/platform-hostname';
import { isPlatformTenantSlug, platformTenantFromEnv } from '@/app/lib/platform-tenant';
import { clerkSignInPageUrl } from '@/app/lib/tenant-auth-urls';
import { resolveTenantSessionFromRequest } from '@/app/lib/tenant-session';
import { tenantCanonicalHostname, tenantCanonicalOrigin } from '@/app/lib/tenant-auth-urls';
import {
  canAccessIntranetPath,
  isOrgAdminFromSession,
} from '@/app/lib/intranet-modules';
import { hasMasterRole } from '@/app/lib/intranet-role-utils';
import {
  intranetRolesFromMetadata,
  intranetRolesFromSessionClaims,
  publicMetadataFromSessionClaims,
} from '@/app/lib/intranet-roles';
import { contentSecurityPolicyHeaderValue, crossOriginOpenerPolicyHeaderValue } from '@/app/lib/content-security-policy';
import { isTenantAccessBlocked } from '@/app/lib/tenant-billing-types';

const isPublicRoute = createRouteMatcher([
  '/',
  '/rentree(.*)',
  '/documents/rentree(.*)',
  '/simulateurTarifs(.*)',
  '/simulateurFournitures(.*)',
  '/portes-ouvertes(.*)',
  '/repartition-classes(.*)',
  '/api/toolbox/public',
  '/api/toolbox/class-allocation/public(.*)',
  '/api/rentree/file',
  '/api/fournitures/file',
  '/api/portes-ouvertes/register',
  '/faire-une-demande(.*)',
  '/demande/merci',
  '/onboarding-rh(.*)',
  '/api/rh/onboarding/public(.*)',
  '/api/agentIAOCR/batch-job/internal-run',
  '/api/travels/ingest-from-email',
  '/api/travels/poll-email',
  '/api/requests/create',
  '/api/requests/confirm',
  '/api/supplies/send',
  '/api/supplies/pdf',
  '/api/chatbot',
  '/api/site/public',
  '/api/tenant/public',
  '/api/tenants/public',
  '/connexion',
  '/plateforme',
  '/sign-in(.*)',
  '/sign-up(.*)',
  '/sign-out(.*)',
  '/mentions-legales',
  '/tarifs',
  '/internat/autorisation(.*)',
  '/api/internat/outings/decision(.*)',
  '/stages/eleve(.*)',
  '/stages/deposer(.*)',
  '/stages/signer(.*)',
  '/stages/candidater(.*)',
  '/api/stages/public(.*)',
  '/certificates/verify(.*)',
  '/api/certificates/verify(.*)',
]);

function localDevTenantSlugFromRequest(request: NextRequest): string | null {
  const fromQuery = request.nextUrl.searchParams.get(LOCAL_DEV_TENANT_QUERY)?.trim();
  if (fromQuery) return fromQuery;
  return request.cookies.get(LOCAL_DEV_TENANT_COOKIE)?.value?.trim() || null;
}

function withOptionalDevTenantCookie(
  response: NextResponse,
  request: NextRequest,
  host: string,
): NextResponse {
  if (!isLocalDevHostname(host)) return response;
  const slug = request.nextUrl.searchParams.get(LOCAL_DEV_TENANT_QUERY)?.trim();
  if (!slug) return response;
  response.cookies.set(LOCAL_DEV_TENANT_COOKIE, slug, {
    path: "/",
    maxAge: 60 * 60 * 24 * 30,
    sameSite: "lax",
    httpOnly: false,
  });
  return response;
}

async function resolveTenantForProxy(request: NextRequest): Promise<TenantConfig> {
  await warmTenantRegistry();
  const host =
    request.headers.get("x-forwarded-host") ||
    request.headers.get("host") ||
    request.nextUrl.hostname;
  const devSlug = localDevTenantSlugFromRequest(request);
  const cached = resolveTenantByHostnameSync(host, devSlug);
  if (cached) return cached;
  try {
    return await resolveTenantByHostname(host, devSlug);
  } catch {
    const normalized = normalizeHostname(host);
    if (isPlatformHostname(normalized)) {
      return platformTenantFromEnv();
    }
    if (isLocalDevHostname(normalized)) {
      const cached = getCachedTenants();
      if (cached?.length) {
        const devTenant = resolveLocalDevTenantBySlug(cached, devSlug);
        if (devTenant) return devTenant;
      }
    }
    if (!process.env.REGISTRY_BUCKET?.trim() && !process.env.TENANT_INDEX_JSON?.trim()) {
      return defaultTenantFromEnv();
    }
    throw new Error(`Domaine non configuré : ${normalized}`);
  }
}

function withTenantHeaders(
  response: NextResponse,
  tenant: TenantConfig,
  pathname?: string,
): NextResponse {
  response.headers.set(TENANT_SLUG_HEADER, tenant.slug);
  response.headers.set("x-tenant-bucket", tenant.dataBucket);
  const omitCsp =
    pathname?.startsWith("/api/rentree/file") ||
    pathname?.startsWith("/api/fournitures/file") ||
    pathname?.startsWith("/documents/rentree/");
  if (!omitCsp) {
    response.headers.set("Content-Security-Policy", contentSecurityPolicyHeaderValue());
    response.headers.set("Cross-Origin-Opener-Policy", crossOriginOpenerPolicyHeaderValue());
  }
  return response;
}

function nextWithTenant(request: NextRequest, tenant: TenantConfig): NextResponse {
  const requestHeaders = new Headers(request.headers);
  requestHeaders.set(TENANT_SLUG_HEADER, tenant.slug);
  requestHeaders.set(TENANT_REQUEST_URL_HEADER, request.nextUrl.pathname + request.nextUrl.search);
  return withTenantHeaders(
    NextResponse.next({
      request: { headers: requestHeaders },
    }),
    tenant,
    request.nextUrl.pathname,
  );
}

function clerkClientForProxy(tenant: TenantConfig, hostname: string) {
  const keys = resolveClerkKeysForHostname(hostname, {
    clerkPublishableKey: tenant.clerkPublishableKey,
    clerkSecretKey: tenant.clerkSecretKey,
    clerkDevPublishableKey: tenant.secrets?.clerkDevPublishableKey,
    clerkDevSecretKey: tenant.secrets?.clerkDevSecretKey,
  });
  const secretKey = keys.secretKey
    ?? (isMultiTenantEnabled() ? tenant.clerkSecretKey : process.env.CLERK_SECRET_KEY);
  if (!secretKey?.trim()) {
    throw new Error("CLERK_SECRET_KEY manquante");
  }
  return createClerkClient({ secretKey });
}

type ProxyAuthState = {
  userId: string;
  orgRole?: string | null;
  sessionClaims: unknown;
};

async function resolveIntranetRolesForProxy(
  authState: ProxyAuthState,
  tenant: TenantConfig,
  hostname: string,
): Promise<{ roles: string[]; publicMetadata: Record<string, unknown> | undefined }> {
  const claims = authState.sessionClaims as Record<string, unknown> | undefined;
  let publicMetadata = publicMetadataFromSessionClaims(claims);
  let roles = intranetRolesFromSessionClaims(claims);

  if (roles.length === 0 && authState.userId) {
    try {
      const user = await clerkClientForProxy(tenant, hostname).users.getUser(authState.userId);
      publicMetadata = user.publicMetadata as Record<string, unknown>;
      roles = intranetRolesFromMetadata(publicMetadata);
    } catch {
      // Session valide mais métadonnées indisponibles — accès module refusé.
    }
  }

  return { roles, publicMetadata };
}

function redirectToTenantCanonicalHost(
  request: NextRequest,
  tenant: TenantConfig,
  host: string,
): NextResponse | null {
  if (isLocalDevHostname(host)) return null;

  const normalizedHost = normalizeHostname(host);

  // Déjà sur un hôte légitime du tenant (ex. lp.docslapro.com) → aucune
  // redirection. Sinon on enverrait l'utilisateur cross-origin et le fetch RSC
  // de Next serait bloqué (« access control checks ») → page blanche.
  if (tenant.hostnames.some((h) => normalizeHostname(h) === normalizedHost)) return null;

  const canonicalHost = tenantCanonicalHostname(tenant);
  if (!canonicalHost || normalizedHost === canonicalHost) return null;

  const dest = new URL(`${request.nextUrl.pathname}${request.nextUrl.search}`, tenantCanonicalOrigin(tenant));
  return NextResponse.redirect(dest);
}

function platformAppOriginFromEnv(): string {
  const raw =
    process.env.PLATFORM_APP_URL?.trim() ||
    process.env.NEXT_PUBLIC_APP_URL?.trim() ||
    "https://scolia.fr";
  try {
    const withScheme = raw.startsWith("http") ? raw : `https://${raw}`;
    return new URL(withScheme).origin;
  } catch {
    return "https://scolia.fr";
  }
}

function isBillingExemptPath(pathname: string): boolean {
  const prefixes = [
    "/abonnement-suspendu",
    "/api/billing/tenant/status",
    "/api/assistance",
    "/sign-out",
  ];
  return prefixes.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

async function handleProxyRequest(
  request: NextRequest,
  auth?: ClerkMiddlewareAuth,
): Promise<NextResponse> {
  let tenant: TenantConfig;
  try {
    tenant = await resolveTenantForProxy(request);
  } catch (err) {
    const message = err instanceof Error ? err.message : "Tenant inconnu";
    if (request.nextUrl.pathname.startsWith("/api/")) {
      return NextResponse.json({ error: message }, { status: 404 });
    }
    return new NextResponse(message, { status: 404 });
  }
  const pathname = request.nextUrl.pathname;
  const host = normalizeHostname(
    request.headers.get("x-forwarded-host") ||
      request.headers.get("host") ||
      request.nextUrl.hostname,
  );

  if (pathname === "/connexion" && !isPlatformHostname(host) && !isLocalDevHostname(host)) {
    return NextResponse.redirect(new URL("/connexion", platformAppOriginFromEnv()));
  }

  if (pathname.startsWith("/sign-in") && !isPlatformHostname(host) && !isLocalDevHostname(host)) {
    const redirectTarget = request.nextUrl.searchParams.get("redirect_url") ?? "";
    if (redirectTarget.includes("/plateforme")) {
      const dest = new URL("/sign-in", platformAppOriginFromEnv());
      dest.searchParams.set("redirect_url", "/plateforme");
      return NextResponse.redirect(dest);
    }
  }

  if (
    (pathname.startsWith("/sign-in") || pathname.startsWith("/sign-up")) &&
    !isPlatformTenantSlug(tenant.slug)
  ) {
    const canonicalRedirect = redirectToTenantCanonicalHost(request, tenant, host);
    if (canonicalRedirect) return withTenantHeaders(canonicalRedirect, tenant);
  }

  if (isPublicRoute(request)) {
    const res = nextWithTenant(request, tenant);
    return withOptionalDevTenantCookie(res, request, host);
  }

  let authState: ProxyAuthState;
  if (isMultiTenantEnabled()) {
    let tenantSession: { userId: string } | null = null;
    try {
      tenantSession = await resolveTenantSessionFromRequest(request, tenant);
    } catch (error) {
      console.error("[proxy] resolveTenantSessionFromRequest", error);
    }
    if (!tenantSession) {
      if (pathname.startsWith("/api/")) {
        return withTenantHeaders(
          NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }),
          tenant,
        );
      }
      return withOptionalDevTenantCookie(
        withTenantHeaders(
          NextResponse.redirect(new URL(clerkSignInPageUrl(tenant, host))),
          tenant,
        ),
        request,
        host,
      );
    }
    authState = { userId: tenantSession.userId, orgRole: null, sessionClaims: undefined };
  } else {
    if (!auth) {
      return withTenantHeaders(
        NextResponse.json({ error: "Auth middleware indisponible." }, { status: 500 }),
        tenant,
      );
    }
    const protectedAuth = await auth.protect();
    authState = {
      userId: protectedAuth.userId,
      orgRole: protectedAuth.orgRole,
      sessionClaims: protectedAuth.sessionClaims,
    };
  }

  const { roles, publicMetadata } = await resolveIntranetRolesForProxy(
    authState,
    tenant,
    request.nextUrl.hostname,
  );
  const isOrgAdmin = isOrgAdminFromSession(authState.orgRole, publicMetadata);

  if (
    !isPlatformTenantSlug(tenant.slug) &&
    isTenantAccessBlocked(tenant.billing?.status) &&
    !isBillingExemptPath(pathname)
  ) {
    if (pathname.startsWith("/api/")) {
      return withTenantHeaders(
        NextResponse.json(
          {
            error: "Abonnement suspendu. Contactez Scola pour régulariser votre situation.",
            code: "SUBSCRIPTION_SUSPENDED",
          },
          { status: 402 },
        ),
        tenant,
      );
    }
    if (pathname !== "/abonnement-suspendu") {
      return withOptionalDevTenantCookie(
        withTenantHeaders(NextResponse.redirect(new URL("/abonnement-suspendu", request.url)), tenant),
        request,
        host,
      );
    }
  }

  const canonicalRedirect = redirectToTenantCanonicalHost(request, tenant, host);
  if (canonicalRedirect) return withTenantHeaders(canonicalRedirect, tenant);

  if (isPlatformHostname(host) && pathname === "/dashboard") {
    const dest = hasMasterRole(roles) ? "/plateforme" : "/";
    return NextResponse.redirect(new URL(dest, request.url));
  }

  if (!canAccessIntranetPath(pathname, roles, isOrgAdmin)) {
    if (pathname.startsWith("/api/")) {
      return NextResponse.json(
        { error: "Accès refusé à ce module.", code: "MODULE_FORBIDDEN" },
        { status: 403 },
      );
    }
    const fallback = "/dashboard";
    return NextResponse.redirect(new URL(fallback, request.url));
  }

  return nextWithTenant(request, tenant);
}

async function clerkAuthHandler(auth: ClerkMiddlewareAuth, request: NextRequest) {
  return handleProxyRequest(request, auth);
}

/**
 * Clés Clerk dynamiques par tenant : indispensable pour que clerkMiddleware()
 * tourne sur chaque requête (sinon <ClerkProvider> / <SignIn> plantent au rendu
 * des Server Components → 500 sur tout le site).
 */
const clerkOptionsForTenant = async (request: NextRequest) => {
  try {
    const host =
      request.headers.get("x-forwarded-host") ||
      request.headers.get("host") ||
      request.nextUrl.hostname;
    const tenant = await resolveTenantForProxy(request);
    const keys = resolveClerkKeysForHostname(host, {
      clerkPublishableKey: tenant.clerkPublishableKey,
      clerkSecretKey: tenant.clerkSecretKey,
      clerkDevPublishableKey: tenant.secrets?.clerkDevPublishableKey,
      clerkDevSecretKey: tenant.secrets?.clerkDevSecretKey,
    });
    if (keys.publishableKey?.trim() && keys.secretKey?.trim()) {
      return { publishableKey: keys.publishableKey, secretKey: keys.secretKey };
    }
  } catch (error) {
    console.error("[proxy:clerkOptions]", error);
  }
  return {};
};

const clerkMw = clerkMiddleware(clerkAuthHandler, clerkOptionsForTenant);

/**
 * Multi-tenant :
 * - Routes API → handler natif (auth via clé secrète tenant). Elles n'affichent
 *   pas <ClerkProvider>, donc pas besoin de clerkMiddleware ni de
 *   CLERK_ENCRYPTION_KEY (absent au runtime Amplify) — ce qui causait des 500
 *   sur toutes les API une fois connecté.
 * - Routes pages → clerkMiddleware (nécessaire au rendu des Server Components).
 */
async function multiTenantMiddleware(
  request: NextRequest,
  event: NextFetchEvent,
): Promise<NextResponse> {
  if (request.nextUrl.pathname.startsWith("/api/")) {
    try {
      return await handleProxyRequest(request);
    } catch (error) {
      console.error("[proxy:api]", error);
      return NextResponse.json(
        {
          error: "Erreur middleware tenant.",
          detail: error instanceof Error ? error.message : String(error),
        },
        { status: 500 },
      );
    }
  }
  return (await clerkMw(request, event)) as NextResponse;
}

export default isMultiTenantEnabled() ? multiTenantMiddleware : clerkMw;

export const config = {
  matcher: [
    '/((?!_next/static|_next/image|favicon\\.ico|.*\\.(?:png|jpg|jpeg|gif|webp|svg|ico|woff2?|ttf|eot|otf|mp4|mp3|pdf)).*)',
    '/documents/rentree/:path*',
    '/',
    '/(api|trpc)(.*)',
  ],
};
