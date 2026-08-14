export type TenantBillingStatus = "active" | "past_due" | "suspended" | "cancelled";

type MicrosoftLicensesBillingStatus = "active" | "suspend_requested" | "revoked";

export type TenantBillingAuditEntry = {
  at: string;
  action: string;
  detail?: string;
  by?: string;
};

export type TenantMicrosoftLicensesBilling = {
  status: MicrosoftLicensesBillingStatus;
  suspendRequestedAt?: string;
  revokedAt?: string;
};

export type TenantBillingState = {
  status: TenantBillingStatus;
  billingMode?: "monthly" | "annual_upfront";
  estimatedStudentCount?: number;
  includedA3Count?: number;
  extraA3Count?: number;
  extraA3UnitMonthlyEur?: number;
  signupRequestId?: string;
  easytransacCustomerId?: string;
  easytransacSubscriptionId?: string;
  easytransacMandateId?: string;
  lastPaymentAt?: string;
  lastPaymentStatus?: string;
  lastFailureAt?: string;
  failureCount?: number;
  graceEndsAt?: string;
  nextBillingAt?: string;
  lastReminderAt?: string;
  reminderStage?: number;
  adminEmail?: string;
  suspendedAt?: string;
  suspendedReason?: string;
  microsoftLicenses?: TenantMicrosoftLicensesBilling;
  auditLog?: TenantBillingAuditEntry[];
};

export const BILLING_STATUS_LABELS: Record<TenantBillingStatus, string> = {
  active: "Actif",
  past_due: "Paiement en retard",
  suspended: "Suspendu",
  cancelled: "Résilié",
};

export const MICROSOFT_LICENSES_BILLING_LABELS: Record<MicrosoftLicensesBillingStatus, string> = {
  active: "Actives",
  suspend_requested: "Suspension demandée",
  revoked: "Révoquées",
};

/** Jours de grâce avant suspension automatique après un échec de paiement. */
export const BILLING_GRACE_DAYS = 30;

/** Jours d'impayé avant demande de révocation des licences Microsoft A3. */
export const MICROSOFT_REVOKE_AFTER_DAYS = 90;

export function isTenantAccessBlocked(status: TenantBillingStatus | undefined): boolean {
  return status === "suspended" || status === "cancelled";
}

function isTenantPastDue(status: TenantBillingStatus | undefined): boolean {
  return status === "past_due";
}
