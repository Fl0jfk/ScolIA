import "server-only";

export {
  getAppSession,
  requireAppUser,
  resolveAppSessionIds,
  resolveSession,
  safeCurrentUser,
  type AppSession,
  type AppUser,
  type AuthSource,
  type CompatAuthUser,
} from "@/app/lib/app-session";

export {
  isOrgAdminFromPublicMetadata,
  isPlatformMasterFromPublicMetadata,
} from "@/app/lib/intranet-auth-metadata";

/** @deprecated Préférer getAppSession(). */
export async function resolveBackedSession(): Promise<{ userId: string } | null> {
  const { resolveSession } = await import("@/app/lib/app-session");
  return resolveSession();
}

/** Compatibilité types — anciennement User session. */
export type { CompatAuthUser as IntranetSessionUser } from "@/app/lib/app-session";
