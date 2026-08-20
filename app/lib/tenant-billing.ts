import "server-only";

import type { TenantSignupRequest } from "@/app/lib/platform-signup-request";
import {
  emailMicrosoftLicensesSuspendRequested,
  emailPaymentFailedAdmin,
  emailPaymentFailedMaster,
  emailPaymentReminderAdmin,
  emailTenantPaymentInvoice,
  emailTenantSuspended,
} from "@/app/lib/tenant-billing-email";
import type {
  TenantBillingAuditEntry,
  TenantBillingState,
} from "@/app/lib/tenant-billing-types";
import {
  BILLING_GRACE_DAYS,
  MICROSOFT_REVOKE_AFTER_DAYS,
} from "@/app/lib/tenant-billing-types";
import {
  EXTRA_A3_PRICE_MONTHLY_EUR,
  INCLUDED_A3_LICENSES,
  normalizeExtraA3Count,
} from "@/app/lib/pricing";
import { patchTenantIndexFields } from "@/app/lib/tenant-registry-admin";
import type { TenantConfig, TenantIndexEntry } from "@/app/lib/tenant-types";

function nowIso(): string {
  return new Date().toISOString();
}

function addDays(iso: string, days: number): string {
  const d = new Date(iso);
  d.setDate(d.getDate() + days);
  return d.toISOString();
}

function appendAudit(
  billing: TenantBillingState,
  entry: Omit<TenantBillingAuditEntry, "at">,
): TenantBillingState {
  const auditLog = [...(billing.auditLog || []), { at: nowIso(), ...entry }];
  return { ...billing, auditLog: auditLog.slice(-50) };
}

function defaultTenantBilling(): TenantBillingState {
  return { status: "active", auditLog: [{ at: nowIso(), action: "initialized" }] };
}

function computeNextBillingAt(
  mode: "monthly" | "annual_upfront" | undefined,
  from = nowIso(),
): string {
  if (mode === "annual_upfront") return addDays(from, 365);
  return addDays(from, 30);
}

export function billingFromSignupRequest(request: TenantSignupRequest): TenantBillingState {
  const at = nowIso();
  const mode = request.billingMode || "monthly";
  return {
    status: "active",
    billingMode: mode,
    estimatedStudentCount: request.establishment.estimatedStudentCount,
    includedA3Count: INCLUDED_A3_LICENSES,
    extraA3Count: normalizeExtraA3Count(request.extraA3Count || 0),
    extraA3UnitMonthlyEur: EXTRA_A3_PRICE_MONTHLY_EUR,
    signupRequestId: request.id,
    easytransacCustomerId: request.easytransac?.customerId,
    easytransacSubscriptionId: request.easytransac?.subscriptionId,
    lastPaymentAt: request.easytransac?.lastPaymentAt || at,
    lastPaymentStatus: request.easytransac?.lastPaymentStatus || "captured",
    adminEmail: request.adminContact.email.trim().toLowerCase(),
    nextBillingAt: computeNextBillingAt(mode, at),
    failureCount: 0,
    ...(request.establishment.wantsMicrosoftLicenses
      ? { microsoftLicenses: { status: "active" as const } }
      : {}),
    auditLog: [{ at, action: "provisioned_from_signup", detail: request.id }],
  };
}

export async function updateTenantExtraA3Count(
  slug: string,
  extraA3Count: number,
  by: string,
): Promise<TenantConfig> {
  const tenant = await loadTenantBySlug(slug);
  const current = getTenantBilling(tenant);
  const next: TenantBillingState = appendAudit(
    {
      ...current,
      includedA3Count: current.includedA3Count ?? INCLUDED_A3_LICENSES,
      extraA3UnitMonthlyEur:
        current.extraA3UnitMonthlyEur ?? EXTRA_A3_PRICE_MONTHLY_EUR,
      extraA3Count: normalizeExtraA3Count(extraA3Count),
    },
    { action: "a3_extra_updated", by, detail: String(normalizeExtraA3Count(extraA3Count)) },
  );
  return saveBilling(slug, next, by);
}

export function getTenantBilling(tenant: TenantIndexEntry | TenantConfig): TenantBillingState {
  return tenant.billing || defaultTenantBilling();
}

async function saveBilling(
  slug: string,
  billing: TenantBillingState,
  by?: string,
): Promise<TenantConfig> {
  return patchTenantIndexFields(slug, { billing }, by);
}

async function loadTenantBySlug(slug: string): Promise<TenantConfig> {
  const { loadAllTenants } = await import("@/app/lib/tenant-registry");
  const tenant = await loadAllTenants().then((list) =>
    list.find((t) => t.slug.toLowerCase() === slug.toLowerCase()),
  );
  if (!tenant) throw new Error(`Tenant « ${slug} » introuvable.`);
  return tenant;
}

export async function recordTenantPaymentSuccess(
  slug: string,
  detail?: { tid?: string; status?: string; amountCents?: number },
): Promise<TenantConfig> {
  const tenant = await loadTenantBySlug(slug);
  const current = getTenantBilling(tenant);
  const tid = detail?.tid?.trim();

  const alreadyRecorded =
    Boolean(tid) &&
    (current.auditLog || []).some((e) => e.action === "payment_success" && e.detail === tid);
  if (alreadyRecorded) {
    return tenant;
  }

  const at = nowIso();
  const next: TenantBillingState = appendAudit(
    {
      ...current,
      status: "active",
      lastPaymentAt: at,
      lastPaymentStatus: detail?.status || "captured",
      lastFailureAt: undefined,
      failureCount: 0,
      graceEndsAt: undefined,
      reminderStage: 0,
      lastReminderAt: undefined,
      suspendedAt: undefined,
      suspendedReason: undefined,
      nextBillingAt: computeNextBillingAt(current.billingMode, at),
      microsoftLicenses:
        current.microsoftLicenses?.status === "suspend_requested" ||
        current.microsoftLicenses?.status === "revoked"
          ? { status: "active" }
          : current.microsoftLicenses,
    },
    { action: "payment_success", detail: tid },
  );
  const updated = await saveBilling(slug, next);

  void emailTenantPaymentInvoice(updated, {
    tid,
    amountCents: detail?.amountCents,
    paidAt: at,
  }).catch((e) => console.error("[tenant-billing] invoice email", slug, e));

  return updated;
}

export async function recordTenantPaymentFailure(
  slug: string,
  detail?: { tid?: string; status?: string; reason?: string },
): Promise<TenantConfig> {
  const tenant = await loadTenantBySlug(slug);
  const current = getTenantBilling(tenant);
  const at = nowIso();
  const failureCount = (current.failureCount || 0) + 1;
  const graceEndsAt = current.graceEndsAt || addDays(at, BILLING_GRACE_DAYS);

  const next: TenantBillingState = appendAudit(
    {
      ...current,
      status: "past_due",
      lastFailureAt: at,
      lastPaymentStatus: detail?.status || "failed",
      failureCount,
      graceEndsAt,
    },
    { action: "payment_failed", detail: detail?.reason || detail?.tid },
  );

  const updated = await saveBilling(slug, next);

  void emailPaymentFailedAdmin(updated, detail?.reason).catch(console.error);
  void emailPaymentFailedMaster(updated, detail?.reason).catch(console.error);

  return updated;
}

async function sendPaymentReminderIfDue(tenant: TenantConfig): Promise<TenantConfig> {
  const billing = getTenantBilling(tenant);
  if (billing.status !== "past_due" || !billing.lastFailureAt) return tenant;

  const daysSinceFailure = Math.floor(
    (Date.now() - new Date(billing.lastFailureAt).getTime()) / (1000 * 60 * 60 * 24),
  );
  const stage =
    daysSinceFailure >= 14 ? 3 : daysSinceFailure >= 7 ? 2 : daysSinceFailure >= 3 ? 1 : 0;
  if (stage === 0 || (billing.reminderStage || 0) >= stage) return tenant;

  const next = appendAudit(
    {
      ...billing,
      reminderStage: stage,
      lastReminderAt: nowIso(),
    },
    { action: "payment_reminder", detail: `stage_${stage}` },
  );
  const updated = await patchTenantIndexFields(tenant.slug, { billing: next });
  void emailPaymentReminderAdmin(updated, stage).catch(console.error);
  return updated;
}

async function maybeAutoSuspendTenant(tenant: TenantConfig): Promise<TenantConfig> {
  const billing = getTenantBilling(tenant);
  if (billing.status !== "past_due" || !billing.graceEndsAt) return tenant;
  if (new Date(billing.graceEndsAt).getTime() > Date.now()) return tenant;
  return suspendTenant(tenant.slug, "system", "Délai de grâce expiré après échec de paiement.");
}

export async function suspendTenant(
  slug: string,
  by: string,
  reason?: string,
): Promise<TenantConfig> {
  const tenant = await loadTenantBySlug(slug);
  const current = getTenantBilling(tenant);
  const at = nowIso();
  const next: TenantBillingState = appendAudit(
    {
      ...current,
      status: "suspended",
      suspendedAt: at,
      suspendedReason: reason?.trim() || undefined,
    },
    { action: "suspended", by, detail: reason },
  );
  const updated = await saveBilling(slug, next, by);
  void emailTenantSuspended(updated, reason).catch(console.error);

  if (
    updated.billing?.microsoftLicenses?.status === "active" &&
    updated.billing.lastFailureAt &&
    daysSince(updated.billing.lastFailureAt) >= MICROSOFT_REVOKE_AFTER_DAYS
  ) {
    await requestMicrosoftLicensesSuspend(slug, by, "Impayé prolongé");
  }

  return updated;
}

export async function reactivateTenant(slug: string, by: string): Promise<TenantConfig> {
  const tenant = await loadTenantBySlug(slug);
  const current = getTenantBilling(tenant);
  const at = nowIso();
  const next: TenantBillingState = appendAudit(
    {
      ...current,
      status: "active",
      lastFailureAt: undefined,
      failureCount: 0,
      graceEndsAt: undefined,
      reminderStage: 0,
      lastReminderAt: undefined,
      suspendedAt: undefined,
      suspendedReason: undefined,
      nextBillingAt: computeNextBillingAt(current.billingMode, at),
      microsoftLicenses: current.microsoftLicenses?.status === "revoked"
        ? current.microsoftLicenses
        : { status: "active" },
    },
    { action: "reactivated", by },
  );
  return saveBilling(slug, next, by);
}

export async function requestMicrosoftLicensesSuspend(
  slug: string,
  by: string,
  reason?: string,
): Promise<TenantConfig> {
  const tenant = await loadTenantBySlug(slug);
  const current = getTenantBilling(tenant);
  if (!current.microsoftLicenses || current.microsoftLicenses.status !== "active") {
    return tenant;
  }
  const at = nowIso();
  const next: TenantBillingState = appendAudit(
    {
      ...current,
      microsoftLicenses: {
        status: "suspend_requested",
        suspendRequestedAt: at,
      },
    },
    { action: "microsoft_suspend_requested", by, detail: reason },
  );
  const updated = await saveBilling(slug, next, by);
  void emailMicrosoftLicensesSuspendRequested(updated, reason).catch(console.error);
  return updated;
}

export async function revokeMicrosoftLicenses(
  slug: string,
  by: string,
): Promise<TenantConfig> {
  const tenant = await loadTenantBySlug(slug);
  const current = getTenantBilling(tenant);
  const at = nowIso();
  const next: TenantBillingState = appendAudit(
    {
      ...current,
      microsoftLicenses: {
        status: "revoked",
        revokedAt: at,
        suspendRequestedAt: current.microsoftLicenses?.suspendRequestedAt,
      },
    },
    { action: "microsoft_revoked", by },
  );
  return saveBilling(slug, next, by);
}

export async function processDunningForAllTenants(): Promise<{
  processed: number;
  suspended: string[];
  reminded: string[];
}> {
  const { loadAllTenants } = await import("@/app/lib/tenant-registry");
  const { isPlatformTenantSlug } = await import("@/app/lib/platform-tenant");
  const tenants = await loadAllTenants();
  const suspended: string[] = [];
  const reminded: string[] = [];

  for (const tenant of tenants) {
    if (isPlatformTenantSlug(tenant.slug)) continue;
    let current = tenant;
    current = await sendPaymentReminderIfDue(current);
    if (current.billing?.lastReminderAt !== tenant.billing?.lastReminderAt) {
      reminded.push(tenant.slug);
    }
    const after = await maybeAutoSuspendTenant(current);
    if (after.billing?.status === "suspended" && tenant.billing?.status !== "suspended") {
      suspended.push(tenant.slug);
    }

    const billing = getTenantBilling(after);
    if (
      billing.status === "past_due" &&
      billing.microsoftLicenses?.status === "active" &&
      billing.lastFailureAt &&
      daysSince(billing.lastFailureAt) >= MICROSOFT_REVOKE_AFTER_DAYS
    ) {
      await requestMicrosoftLicensesSuspend(tenant.slug, "system", "Impayé > 90 jours");
    }
  }

  return { processed: tenants.length, suspended, reminded };
}

function daysSince(iso: string): number {
  return Math.floor((Date.now() - new Date(iso).getTime()) / (1000 * 60 * 60 * 24));
}

export function parseTenantBillingOrderId(
  orderId: string,
): { kind: "signup"; signupId: string; mode?: string } | { kind: "tenant"; slug: string } | null {
  if (orderId.startsWith("scola-tenant:")) {
    const rest = orderId.slice("scola-tenant:".length);
    const slug = rest.split(":")[0];
    if (slug) return { kind: "tenant", slug };
  }
  if (orderId.startsWith("scola-")) {
    const parts = orderId.split("-");
    if (parts.length >= 2 && parts[1] !== "tenant") {
      return { kind: "signup", signupId: parts[1], mode: parts[2] };
    }
  }
  return null;
}

export function tenantRecurringOrderId(slug: string, period = nowIso().slice(0, 7)): string {
  return `scola-tenant:${slug}:${period}`;
}
