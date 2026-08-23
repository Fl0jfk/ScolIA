import "server-only";

export type AuthProviderMode = "better-auth";

export function getAuthProviderMode(): AuthProviderMode {
  const provider = process.env.AUTH_PROVIDER?.trim().toLowerCase();
  if (provider && provider !== "better-auth") {
    throw new Error(
      `AUTH_PROVIDER="${provider}" n'est plus supporté. Utilisez AUTH_PROVIDER=better-auth.`,
    );
  }
  return "better-auth";
}

export function isDatabaseConfigured(): boolean {
  return Boolean(process.env.DATABASE_URL?.trim());
}

export function isBetterAuthConfigured(): boolean {
  getAuthProviderMode();
  return isDatabaseConfigured() && Boolean(process.env.BETTER_AUTH_SECRET?.trim());
}

/** Better-Auth actif si la configuration minimale est présente. */
export function isBetterAuthActive(): boolean {
  return isBetterAuthConfigured();
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
  const configured = isBetterAuthConfigured();
  return {
    provider: "better-auth",
    betterAuthRuntime: configured,
    nextStep: configured
      ? "Better-Auth actif."
      : "Configurer DATABASE_URL + BETTER_AUTH_SECRET + AUTH_PROVIDER=better-auth.",
  };
}
