import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";
import {
  TENANT_SLUG_HEADER,
  TENANT_REQUEST_URL_HEADER,
  type TenantConfig,
} from "@/app/lib/tenant-types";
import {
  defaultTenantFromEnv,
  normalizeHostname,
  resolveLocalDevTenantBySlug,
  resolveTenantByHostname,
  resolveTenantByHostnameSync,
  warmTenantRegistry,
  getCachedTenants,
} from "@/app/lib/tenant-registry";
import { isLocalDevHostname } from "@/app/lib/local-host-keys";
import {
  LOCAL_DEV_TENANT_COOKIE,
  LOCAL_DEV_TENANT_QUERY,
} from "@/app/lib/local-dev";
import { isPlatformHostname } from "@/app/lib/platform-hostname";
import { isPlatformTenantSlug, platformTenantFromEnv } from "@/app/lib/platform-tenant";
import {
  tenantCanonicalHostname,
  tenantCanonicalOrigin,
} from "@/app/lib/tenant-auth-urls";
import { canAccessIntranetPath } from "@/app/lib/intranet-modules";
import {
  isAppOnlyAllowedPath,
  resolveChannelMode,
} from "@/app/lib/channel-access";
import { hasMasterRole } from "@/app/lib/intranet-role-utils";
import { listActiveMembershipsForUser } from "@/app/lib/user-membership";
import {
  contentSecurityPolicyHeaderValue,
  crossOriginOpenerPolicyHeaderValue,
} from "@/app/lib/content-security-policy";
import { isTenantAccessBlocked } from "@/app/lib/tenant-billing-types";
import { PROXY_PUBLIC_ROUTE_MATCHERS } from "@/app/lib/public-routes";
import { legacyDocsLaProRedirect } from "@/app/lib/legacy-hostname-redirects";
import { isBetterAuthActive } from "@/app/lib/auth-config";
import {
  isMustChangePasswordAllowedPath,
  isTwoFactorSetupAllowedPath,
  resolveBetterAuthProxyState,
} from "@/app/lib/proxy-better-auth";
import { assertUserBelongsToTenant } from "@/app/lib/etablissement-db";

async function loadModuleAccessForProxy() {
  try {
    const { loadModuleAccess } = await import("@/app/lib/module-access-store");
    return await loadModuleAccess();
  } catch {
    return null;
  }
}

/**
 * Équivalent `createRouteMatcher` : patterns style `/path(.*)`.
 */
function createPublicRouteMatcher(patterns: readonly string[]) {
  const regexes = patterns.map((pattern) => {
    const source = pattern.replace(/(\(\.\*\))|([.+?^${}()|[\]\\])/g, (match, wildcard) => {
      if (wildcard) return ".*";
      return `\\${match}`;
    });
    return new RegExp(`^${source}$`);
  });
  return (request: NextRequest): boolean => {
    const pathname = request.nextUrl.pathname;
    return regexes.some((re) => re.test(pathname));
  };
}

const isPublicRoute = createPublicRouteMatcher(PROXY_PUBLIC_ROUTE_MATCHERS);

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
      const list = getCachedTenants();
      if (list?.length) {
        const devTenant = resolveLocalDevTenantBySlug(list, devSlug);
        if (devTenant) return devTenant;
      }
    }
    if (!process.env.REGISTRY_BUCKET?.trim() && !process.env.TENANT_INDEX_JSON?.trim()) {
      return defaultTenantFromEnv();
    }
    throw new Error(`Domaine non configuré : ${normalized}`);
  }
}

function createCspNonce(): string {
  return Buffer.from(crypto.randomUUID()).toString("base64");
}

function withTenantHeaders(
  response: NextResponse,
  tenant: TenantConfig,
  pathname?: string,
  nonce?: string,
): NextResponse {
  response.headers.set(TENANT_SLUG_HEADER, tenant.slug);
  response.headers.set("x-tenant-bucket", tenant.dataBucket);
  const omitCsp =
    pathname?.startsWith("/api/rentree/file") ||
    pathname?.startsWith("/api/fournitures/file") ||
    pathname?.startsWith("/documents/rentree/");
  if (!omitCsp) {
    const n = nonce ?? createCspNonce();
    response.headers.set("Content-Security-Policy", contentSecurityPolicyHeaderValue(n));
    response.headers.set("Cross-Origin-Opener-Policy", crossOriginOpenerPolicyHeaderValue());
  }
  return response;
}

function nextWithTenant(request: NextRequest, tenant: TenantConfig): NextResponse {
  const nonce = createCspNonce();
  const requestHeaders = new Headers(request.headers);
  requestHeaders.set(TENANT_SLUG_HEADER, tenant.slug);
  requestHeaders.set(TENANT_REQUEST_URL_HEADER, request.nextUrl.pathname + request.nextUrl.search);
  requestHeaders.set("x-nonce", nonce);
  return withTenantHeaders(
    NextResponse.next({
      request: { headers: requestHeaders },
    }),
    tenant,
    request.nextUrl.pathname,
    nonce,
  );
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

  const dest = new URL(
    `${request.nextUrl.pathname}${request.nextUrl.search}`,
    tenantCanonicalOrigin(tenant),
  );
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

function unauthorizedResponse(
  request: NextRequest,
  tenant: TenantConfig,
  host: string,
): NextResponse {
  const pathname = request.nextUrl.pathname;
  if (pathname.startsWith("/api/")) {
    return withTenantHeaders(
      NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }),
      tenant,
    );
  }
  const signInUrl = new URL("/auth/sign-in", request.url);
  signInUrl.searchParams.set("redirect_url", `${pathname}${request.nextUrl.search}`);
  return withOptionalDevTenantCookie(
    withTenantHeaders(NextResponse.redirect(signInUrl), tenant),
    request,
    host,
  );
}

function authUnavailableResponse(
  request: NextRequest,
  tenant: TenantConfig,
): NextResponse {
  const message =
    "Authentification indisponible. Better-Auth n'est pas configuré (DATABASE_URL / BETTER_AUTH_SECRET).";
  if (request.nextUrl.pathname.startsWith("/api/")) {
    return withTenantHeaders(
      NextResponse.json({ error: message, code: "AUTH_UNAVAILABLE" }, { status: 503 }),
      tenant,
    );
  }
  return withTenantHeaders(new NextResponse(message, { status: 503 }), tenant);
}

async function handleProxyRequest(request: NextRequest): Promise<NextResponse> {
  const legacyRedirect = legacyDocsLaProRedirect(request);
  if (legacyRedirect) return legacyRedirect;

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

  if (!isBetterAuthActive()) {
    return authUnavailableResponse(request, tenant);
  }

  const betterAuthState = await resolveBetterAuthProxyState(request, tenant);
  if (!betterAuthState) {
    return unauthorizedResponse(request, tenant, host);
  }

  const method = request.method.toUpperCase();
  const isSafeMethod = method === "GET" || method === "HEAD" || method === "OPTIONS";

  const {
    readSupervisionCookieFromRequest,
    resolveActiveSupervision,
    isSupervisionApiPath,
    isSupervisionWriteAllowedPath,
  } = await import("@/app/lib/supervision");

  const supervisionCookie = readSupervisionCookieFromRequest(request);
  let supervisionActive:
    | Awaited<ReturnType<typeof resolveActiveSupervision>>
    | null = null;
  if (supervisionCookie) {
    supervisionActive = await resolveActiveSupervision({
      actorUserId: betterAuthState.authUserId,
      cookie: supervisionCookie,
    });
  }

  if (supervisionActive && !isSafeMethod && !isSupervisionWriteAllowedPath(pathname)) {
    return withTenantHeaders(
      NextResponse.json(
        {
          error: "Mode supervision : lecture seule. Quittez la supervision pour modifier.",
          code: "SUPERVISION_READ_ONLY",
        },
        { status: 403 },
      ),
      tenant,
    );
  }

  const viewRoles = supervisionActive?.target.roles ?? betterAuthState.roles;
  const viewIsOrgAdmin = supervisionActive
    ? supervisionActive.target.orgAdmin || supervisionActive.target.platformAdmin
    : betterAuthState.orgAdmin || betterAuthState.platformAdmin;
  const viewUserId = supervisionActive?.target.userId ?? betterAuthState.authUserId;
  const viewBusinessUserId =
    supervisionActive?.target.businessUserId ?? betterAuthState.userId;
  const viewEmail = supervisionActive?.target.email ?? betterAuthState.email;

  const roles = betterAuthState.roles;

  const [tenantGate, memberships] = await Promise.all([
    assertUserBelongsToTenant({
      userId: betterAuthState.authUserId,
      userEtablissementId: betterAuthState.homeEtablissementId,
      platformAdmin: betterAuthState.platformAdmin,
      tenant,
    }),
    listActiveMembershipsForUser(betterAuthState.authUserId),
  ]);
  if (!tenantGate.ok) {
    if (pathname.startsWith("/api/")) {
      return withTenantHeaders(
        NextResponse.json(
          { error: tenantGate.message, code: tenantGate.code },
          { status: 403 },
        ),
        tenant,
      );
    }
    return withOptionalDevTenantCookie(
      withTenantHeaders(
        NextResponse.redirect(new URL("/connexion", platformAppOriginFromEnv())),
        tenant,
      ),
      request,
      host,
    );
  }

  // Après matérialisation éventuelle du membership staff (legacy).
  const hasStaffMembership = memberships.some((m) => m.context === "staff");
  const hasParentOrEleveSignal = memberships.some(
    (m) => m.context === "parent" || m.context === "eleve",
  );
  const channelMode = resolveChannelMode({
    roles,
    platformAdmin: betterAuthState.platformAdmin,
    orgAdmin: betterAuthState.orgAdmin,
    hasStaffMembership,
    hasParentOrEleveSignal,
  });

  if (channelMode === "app_only" && !isAppOnlyAllowedPath(pathname)) {
    if (pathname.startsWith("/api/")) {
      return withTenantHeaders(
        NextResponse.json(
          {
            error: "Accès réservé à l’application mobile (canal famille / élève).",
            code: "APP_ONLY_CHANNEL",
          },
          { status: 403 },
        ),
        tenant,
      );
    }
    return withOptionalDevTenantCookie(
      withTenantHeaders(NextResponse.redirect(new URL("/app-mobile", request.url)), tenant),
      request,
      host,
    );
  }

  if (
    betterAuthState.mustChangePassword &&
    !isMustChangePasswordAllowedPath(pathname) &&
    !isPublicRoute(request)
  ) {
    if (pathname.startsWith("/api/")) {
      return withTenantHeaders(
        NextResponse.json(
          {
            error: "Changement de mot de passe obligatoire.",
            code: "MUST_CHANGE_PASSWORD",
          },
          { status: 403 },
        ),
        tenant,
      );
    }
    const dest = new URL("/auth/change-password-required", request.url);
    dest.searchParams.set("redirect_url", `${pathname}${request.nextUrl.search}`);
    return withOptionalDevTenantCookie(
      withTenantHeaders(NextResponse.redirect(dest), tenant),
      request,
      host,
    );
  }

  if (
    betterAuthState.requiresTwoFactorSetup &&
    !isTwoFactorSetupAllowedPath(pathname) &&
    !isPublicRoute(request)
  ) {
    if (pathname.startsWith("/api/")) {
      return withTenantHeaders(
        NextResponse.json(
          {
            error: "Activation de la double authentification obligatoire.",
            code: "MUST_SETUP_2FA",
          },
          { status: 403 },
        ),
        tenant,
      );
    }
    const dest = new URL("/auth/setup-2fa", request.url);
    dest.searchParams.set("redirect_url", `${pathname}${request.nextUrl.search}`);
    return withOptionalDevTenantCookie(
      withTenantHeaders(NextResponse.redirect(dest), tenant),
      request,
      host,
    );
  }

  if (
    !isPlatformTenantSlug(tenant.slug) &&
    isTenantAccessBlocked(tenant.billing?.status) &&
    !isBillingExemptPath(pathname)
  ) {
    if (pathname.startsWith("/api/")) {
      return withTenantHeaders(
        NextResponse.json(
          {
            error: "Abonnement suspendu. Contactez ScolIA pour régulariser votre situation.",
            code: "SUBSCRIPTION_SUSPENDED",
          },
          { status: 402 },
        ),
        tenant,
      );
    }
    if (pathname !== "/abonnement-suspendu") {
      return withOptionalDevTenantCookie(
        withTenantHeaders(
          NextResponse.redirect(new URL("/abonnement-suspendu", request.url)),
          tenant,
        ),
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

  if (isSupervisionApiPath(pathname)) {
    return withOptionalDevTenantCookie(nextWithTenant(request, tenant), request, host);
  }

  let pathAllowed = false;
  try {
    pathAllowed = canAccessIntranetPath(
      pathname,
      viewRoles,
      viewIsOrgAdmin,
      await loadModuleAccessForProxy(),
      {
        userId: viewUserId,
        businessUserId: viewBusinessUserId,
      },
    );
  } catch (pathErr) {
    console.error("[proxy] canAccessIntranetPath", pathname, pathErr);
    // En cas de bug droits (ex. récursion), ne pas renvoyer une 500 HTML opaque :
    // laisser passer uniquement si orgAdmin / admin, sinon refuser proprement.
    pathAllowed = viewIsOrgAdmin || viewRoles.includes("admin") || hasMasterRole(viewRoles);
  }
  let photocopiesOpsBypass = false;
  if (
    !pathAllowed &&
    (pathname === "/photocopies-couleur" ||
      pathname.startsWith("/photocopies-couleur/") ||
      pathname === "/api/photocopies-couleur" ||
      pathname.startsWith("/api/photocopies-couleur/"))
  ) {
    try {
      const { loadAppConfig } = await import("@/app/lib/app-config");
      const { resolvePhotocopiesOpsEmails } = await import("@/app/lib/photocopies-couleur-ops");
      const { isPhotocopiesOpsHandlerResolved } = await import(
        "@/app/lib/photocopies-couleur-ops-server"
      );
      const { loadModuleAccess } = await import("@/app/lib/module-access-store");
      const bundle = await loadAppConfig();
      const access = await loadModuleAccess().catch(() => null);
      photocopiesOpsBypass = isPhotocopiesOpsHandlerResolved({
        email: viewEmail,
        opsEmails: resolvePhotocopiesOpsEmails(bundle.notifications),
        moduleAccess: access,
        lookup: {
          userId: viewUserId,
          businessUserId: viewBusinessUserId,
        },
        roles: viewRoles,
      });
    } catch {
      photocopiesOpsBypass = false;
    }
  }

  if (
    !pathAllowed &&
    !photocopiesOpsBypass &&
    !isMustChangePasswordAllowedPath(pathname) &&
    !isTwoFactorSetupAllowedPath(pathname)
  ) {
    if (pathname.startsWith("/api/")) {
      return NextResponse.json(
        { error: "Accès refusé à ce module.", code: "MODULE_FORBIDDEN" },
        { status: 403 },
      );
    }
    return NextResponse.redirect(new URL("/dashboard", request.url));
  }

  return withOptionalDevTenantCookie(nextWithTenant(request, tenant), request, host);
}

async function multiTenantMiddleware(request: NextRequest): Promise<NextResponse> {
  try {
    return await handleProxyRequest(request);
  } catch (error) {
    console.error("[proxy]", error);
    if (request.nextUrl.pathname.startsWith("/api/")) {
      return NextResponse.json(
        {
          error: "Erreur middleware tenant.",
          detail: error instanceof Error ? error.message : String(error),
        },
        { status: 500 },
      );
    }
    return new NextResponse(
      error instanceof Error ? error.message : "Erreur middleware",
      { status: 500 },
    );
  }
}

export default multiTenantMiddleware;

export const config = {
  matcher: [
    // Exclure assets statiques (dont .mjs : worker pdf.js — sinon HTML de login → MIME error)
    "/((?!_next/static|_next/image|favicon\\.ico|.*\\.(?:png|jpg|jpeg|gif|webp|svg|ico|woff2?|ttf|eot|otf|mp4|mp3|pdf|mjs|js|css|map)).*)",
    "/documents/rentree/:path*",
    "/",
    "/(api|trpc)(.*)",
  ],
};
