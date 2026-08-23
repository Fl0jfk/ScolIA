import "server-only";

import type { NextRequest } from "next/server";
import type { TenantConfig } from "@/app/lib/tenant-types";
import {
  resolveAppSessionIds,
  safeCurrentUser,
  type CompatAuthUser,
} from "@/app/lib/app-session";

/** Session tenant — délègue à Better-Auth (plus auth legacy authenticateRequest). */
export async function resolveTenantSessionFromRequest(
  _request: NextRequest,
  _tenant: TenantConfig,
): Promise<{ userId: string } | null> {
  return resolveAppSessionIds();
}

export async function resolveTenantSession(): Promise<{ userId: string } | null> {
  return resolveAppSessionIds();
}

/** Profil utilisateur courant (forme session unifiée). */
export async function resolveTenantCurrentUser(): Promise<CompatAuthUser | null> {
  return safeCurrentUser();
}

export async function getTenantCurrentUser(): Promise<CompatAuthUser | null> {
  return safeCurrentUser();
}
