import "server-only";

import { createClerkClient, type User } from "@clerk/backend";
import { headers } from "next/headers";
import type { NextRequest } from "next/server";
import { isLocalDevHostname, resolveClerkKeysForHostname } from "@/app/lib/clerk-tenant-keys";
import { requestOriginFromHostHeader } from "@/app/lib/local-dev";
import { getTenant } from "@/app/lib/tenant-context";
import { TENANT_REQUEST_URL_HEADER } from "@/app/lib/tenant-types";
import type { TenantConfig } from "@/app/lib/tenant-types";
import { normalizeHostname } from "@/app/lib/tenant-registry";

function requestHostHeader(hdrs: Headers): string {
  return (hdrs.get("x-forwarded-host") || hdrs.get("host") || "localhost").trim();
}

function clerkAuthorizedParties(hostHeader: string): string[] | undefined {
  const raw = hostHeader.trim();
  if (!raw) return undefined;
  const normalized = normalizeHostname(raw);
  const parties = new Set<string>();
  if (isLocalDevHostname(normalized)) {
    parties.add(requestOriginFromHostHeader(raw));
  }
  parties.add(`https://${normalized}`);
  if (!isLocalDevHostname(normalized)) {
    parties.add(`http://${normalized}`);
  }
  return [...parties];
}

function clerkKeysForTenant(tenant: TenantConfig, hostHeader: string) {
  return resolveClerkKeysForHostname(hostHeader, {
    clerkPublishableKey: tenant.clerkPublishableKey,
    clerkSecretKey: tenant.clerkSecretKey,
    clerkDevPublishableKey: tenant.secrets?.clerkDevPublishableKey,
    clerkDevSecretKey: tenant.secrets?.clerkDevSecretKey,
  });
}

function clerkRequestFromHeaders(hdrs: Headers, fallbackUrl?: string): Request {
  const headerInit = new Headers();
  hdrs.forEach((value, key) => {
    headerInit.set(key, value);
  });

  const hostHeader = requestHostHeader(hdrs);
  const origin = requestOriginFromHostHeader(hostHeader);

  const explicitUrl = hdrs.get(TENANT_REQUEST_URL_HEADER)?.trim();
  if (explicitUrl) {
    const absolutePath = explicitUrl.startsWith("http")
      ? explicitUrl
      : `${origin}${explicitUrl.startsWith("/") ? explicitUrl : `/${explicitUrl}`}`;
    return new Request(absolutePath, { headers: headerInit });
  }

  const path =
    hdrs.get("x-invoke-path") ||
    hdrs.get("next-url") ||
    (fallbackUrl ? new URL(fallbackUrl).pathname + new URL(fallbackUrl).search : "/");

  const absolutePath = path.startsWith("http") ? path : `${origin}${path.startsWith("/") ? path : `/${path}`}`;

  return new Request(absolutePath, { headers: headerInit });
}

/** Reconstruit la requête HTTP courante (cookies inclus) pour Clerk backend. */
async function incomingClerkRequest(): Promise<Request> {
  const hdrs = await headers();
  return clerkRequestFromHeaders(hdrs);
}

function clerkRequestFromNextRequest(request: NextRequest): Request {
  const headerInit = new Headers();
  request.headers.forEach((value, key) => {
    headerInit.set(key, value);
  });
  return clerkRequestFromHeaders(headerInit, request.url);
}

async function authenticateTenantRequest(
  request: Request,
  tenant: TenantConfig,
): Promise<{ userId: string } | null> {
  const hostHeader =
    request.headers.get("x-forwarded-host") || request.headers.get("host") || "";
  const keys = clerkKeysForTenant(tenant, hostHeader);
  const secretKey = keys.secretKey?.trim();
  if (!secretKey) return null;

  try {
    const clerk = createClerkClient({
      secretKey,
      publishableKey: keys.publishableKey,
    });

    const state = await clerk.authenticateRequest(request, {
      secretKey,
      publishableKey: keys.publishableKey,
      authorizedParties: clerkAuthorizedParties(hostHeader),
    });

    if (state.isAuthenticated) {
      const authObj = state.toAuth();
      if (authObj.userId) return { userId: authObj.userId };
    }
  } catch (error) {
    console.error("[authenticateTenantRequest]", error);
  }

  return null;
}

/** Session via la clé secrète du tenant (middleware + routes API). */
export async function resolveTenantSessionFromRequest(
  request: NextRequest,
  tenant: TenantConfig,
): Promise<{ userId: string } | null> {
  return authenticateTenantRequest(clerkRequestFromNextRequest(request), tenant);
}

/** Session via la clé secrète du tenant (sans clés dynamiques Clerk / CLERK_ENCRYPTION_KEY). */
export async function resolveTenantSession(): Promise<{ userId: string } | null> {
  const tenant = await getTenant();
  const request = await incomingClerkRequest();
  return authenticateTenantRequest(request, tenant);
}

export async function resolveTenantCurrentUser(): Promise<User | null> {
  const session = await resolveTenantSession();
  if (!session) return null;

  const hdrs = await headers();
  const hostHeader = requestHostHeader(hdrs);
  const tenant = await getTenant();
  const keys = clerkKeysForTenant(tenant, hostHeader);
  const clerk = createClerkClient({ secretKey: keys.secretKey });
  try {
    return await clerk.users.getUser(session.userId);
  } catch (error) {
    console.error("[resolveTenantCurrentUser]", error);
    return null;
  }
}
