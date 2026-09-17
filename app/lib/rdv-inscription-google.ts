import "server-only";

import { getTenant } from "@/app/lib/tenant-context";
import {
  getTenantSecrets,
  loadTenantSecretsFile,
  saveTenantSecretsFile,
} from "@/app/lib/tenant-registry";
import type { TenantSecrets } from "@/app/lib/tenant-types";
import {
  getRdvInscriptionGoogleRefreshToken,
  markRdvInscriptionGoogleLinked,
  persistRdvInscriptionGoogleRefreshToken,
} from "@/app/lib/rdv-inscription-db";

export async function saveRdvInscriptionGoogleLinkSecret(input: {
  refreshToken: string;
  linkedEmail?: string | null;
  linkedDisplayName?: string | null;
}): Promise<void> {
  const tenant = await getTenant();
  const existing =
    (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  if (!existing) throw new Error("Secrets tenant introuvables.");

  const linkedAt = new Date().toISOString();
  const refreshToken = input.refreshToken.trim();
  const prevGoogle = existing.google ?? {};
  const google: NonNullable<TenantSecrets["google"]> = {
    ...prevGoogle,
    calendar: {
      refreshToken,
      linkedEmail: input.linkedEmail?.trim() || undefined,
      linkedDisplayName: input.linkedDisplayName?.trim() || undefined,
      linkedAt,
    },
  };

  // BDD d’abord (fiable multi-instances) puis miroir S3.
  await markRdvInscriptionGoogleLinked({
    linked: true,
    email: input.linkedEmail,
    linkedAt: new Date(linkedAt),
    refreshToken,
  });

  try {
    await saveTenantSecretsFile(tenant.slug, { ...existing, google });
  } catch (e) {
    console.error("[rdv-inscription] miroir S3 refresh token échoué (BDD OK):", e);
  }
}

export async function persistRotatedGoogleRefreshToken(refreshToken: string): Promise<void> {
  const trimmed = refreshToken.trim();
  await persistRdvInscriptionGoogleRefreshToken(trimmed);

  const tenant = await getTenant();
  try {
    const existing =
      (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
    if (!existing?.google?.calendar?.refreshToken) return;
    const google: NonNullable<TenantSecrets["google"]> = {
      ...existing.google,
      calendar: {
        ...existing.google.calendar,
        refreshToken: trimmed,
      },
    };
    await saveTenantSecretsFile(tenant.slug, { ...existing, google });
  } catch (e) {
    console.error("[rdv-inscription] rotation S3 refresh token échouée (BDD OK):", e);
  }
}

export async function clearRdvInscriptionGoogleLinkSecret(): Promise<void> {
  const tenant = await getTenant();
  await markRdvInscriptionGoogleLinked({
    linked: false,
    email: null,
    linkedAt: null,
    refreshToken: null,
  });

  try {
    const existing =
      (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
    if (!existing) return;
    const google = existing.google
      ? { ...existing.google, calendar: undefined }
      : undefined;
    await saveTenantSecretsFile(tenant.slug, { ...existing, google });
  } catch (e) {
    console.error("[rdv-inscription] clear S3 refresh token échoué (BDD OK):", e);
  }
}

/** Jeton utilisable pour Google Calendar (BDD prioritaire, puis S3). */
export async function resolveRdvInscriptionGoogleRefreshToken(): Promise<string | null> {
  const fromDb = await getRdvInscriptionGoogleRefreshToken();
  if (fromDb) return fromDb;

  const tenant = await getTenant();
  const secrets =
    (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  return secrets?.google?.calendar?.refreshToken?.trim() || null;
}

export async function getRdvInscriptionGoogleLinkStatus(): Promise<{
  linked: boolean;
  linkedEmail: string | null;
  linkedDisplayName: string | null;
  linkedAt: string | null;
  clientConfigured: boolean;
  /** true seulement s’il existe un refresh token utilisable. */
  tokenReady: boolean;
}> {
  const tenant = await getTenant();
  const secrets =
    (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  const cal = secrets?.google?.calendar;
  const clientId =
    secrets?.google?.clientId?.trim() ||
    process.env.GOOGLE_CLIENT_ID?.trim() ||
    process.env.GOOGLE_CALENDAR_CLIENT_ID?.trim() ||
    "";
  const clientSecret =
    secrets?.google?.clientSecret?.trim() ||
    process.env.GOOGLE_CLIENT_SECRET?.trim() ||
    process.env.GOOGLE_CALENDAR_CLIENT_SECRET?.trim() ||
    "";

  const token = await resolveRdvInscriptionGoogleRefreshToken();
  const tokenReady = Boolean(token);

  let linkedEmail = cal?.linkedEmail?.trim() || null;
  let linkedDisplayName = cal?.linkedDisplayName?.trim() || null;
  let linkedAt = cal?.linkedAt?.trim() || null;

  try {
    const { getRdvInscriptionConfig } = await import("@/app/lib/rdv-inscription-db");
    const config = await getRdvInscriptionConfig();
    linkedEmail = linkedEmail || config.googleLinkedEmail;
    linkedAt = linkedAt || config.googleLinkedAt;
  } catch {
    /* ignore */
  }

  return {
    linked: tokenReady,
    linkedEmail: tokenReady ? linkedEmail : null,
    linkedDisplayName: tokenReady ? linkedDisplayName : null,
    linkedAt: tokenReady ? linkedAt : null,
    clientConfigured: Boolean(clientId && clientSecret),
    tokenReady,
  };
}
