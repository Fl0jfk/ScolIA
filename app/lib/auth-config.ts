import "server-only";

export type AuthProviderMode = "better-auth" | "dual" | "legacy";

export function getAuthProviderMode(): AuthProviderMode {
  const raw = process.env.AUTH_PROVIDER?.trim().toLowerCase();
  if (raw === "better-auth" || raw === "dual" || raw === "legacy") return raw;
  return "dual";
}

export function isDatabaseConfigured(): boolean {
  return Boolean(process.env.DATABASE_URL?.trim());
}

export function isBetterAuthConfigured(): boolean {
  return isDatabaseConfigured() && Boolean(process.env.BETTER_AUTH_SECRET?.trim());
}

export function isBetterAuthActive(): boolean {
  const mode = getAuthProviderMode();
  if (mode === "legacy") return false;
  return isBetterAuthConfigured();
}

export function isLegacyAuthActive(): boolean {
  return getAuthProviderMode() !== "better-auth";
}

/** Chemins pilote Better-Auth (dual mode). */
export function betterAuthPilotPrefixes(): string[] {
  const raw =
    process.env.BETTER_AUTH_PILOT_PATHS?.trim() ||
    "/auth,/api/auth,/dashboard";
  return raw
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

export function isBetterAuthPilotPath(pathname: string): boolean {
  if (getAuthProviderMode() === "better-auth") return true;
  if (getAuthProviderMode() === "legacy") return false;
  return betterAuthPilotPrefixes().some(
    (prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`),
  );
}

export function betterAuthBaseUrl(): string {
  const fromEnv =
    process.env.BETTER_AUTH_URL?.trim() ||
    process.env.NEXT_PUBLIC_APP_URL?.trim() ||
    "http://localhost:3000";
  return fromEnv.replace(/\/$/, "");
}

/** Diagnostic runtime auth (status API). */
export function authProviderStatus(): {
  provider: AuthProviderMode;
  betterAuthRuntime: boolean;
  nextStep: string;
} {
  const provider = getAuthProviderMode();
  return {
    provider,
    betterAuthRuntime: isBetterAuthActive(),
    nextStep:
      provider === "better-auth"
        ? isBetterAuthConfigured()
          ? "Better-Auth actif."
          : "Configurer DATABASE_URL + BETTER_AUTH_SECRET."
        : provider === "dual"
          ? "Élargir BETTER_AUTH_PILOT_PATHS puis basculer AUTH_PROVIDER=better-auth."
          : "Passer AUTH_PROVIDER=better-auth (mode legacy désactivé).",
  };
}
