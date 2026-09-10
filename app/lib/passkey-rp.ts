/**
 * Options RP (Relying Party) pour WebAuthn / passkeys.
 * rpID = domaine enregistrable partagé (scolia.fr) pour tous les sous-domaines.
 */
import { betterAuthBaseUrl } from "@/app/lib/auth-config";

export function resolvePasskeyRpId(): string {
  const fromEnv = process.env.PASSKEY_RP_ID?.trim();
  if (fromEnv) return fromEnv;

  try {
    const host = new URL(betterAuthBaseUrl()).hostname.toLowerCase();
    if (host === "localhost" || host === "127.0.0.1") return "localhost";
    if (host === "scolia.fr" || host.endsWith(".scolia.fr")) return "scolia.fr";
    if (host === "docslapro.com" || host.endsWith(".docslapro.com")) return "docslapro.com";
    return host;
  } catch {
    return "localhost";
  }
}

export function resolvePasskeyRpName(): string {
  return process.env.PASSKEY_RP_NAME?.trim() || "ScolIA";
}
