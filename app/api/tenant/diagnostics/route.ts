import { resolveSession } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveTenantCurrentUser, resolveTenantSession } from "@/app/lib/tenant-session";
import { getJson } from "@/app/lib/s3-storage";
import { getTenant } from "@/app/lib/tenant-context";
import { isMultiTenantEnabled } from "@/app/lib/tenant-registry";
import { clerkFrontendDomainFromPublishableKey } from "@/app/lib/clerk-pk-domain";

/** Diagnostic rapide tenant + Clerk + S3 (admin org). */
export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const report: Record<string, unknown> = {
    ok: true,
    checks: {} as Record<string, unknown>,
  };

  try {
    const tenant = await getTenant();
    report.tenant = {
      slug: tenant.slug,
      dataBucket: tenant.dataBucket,
      clerkFrontendDomain: clerkFrontendDomainFromPublishableKey(tenant.clerkPublishableKey),
      hasClerkSecretKey: Boolean(tenant.clerkSecretKey?.trim()),
      hasAwsOverride: Boolean(tenant.secrets?.aws?.roleArn || tenant.secrets?.aws?.accessKeyId),
    };
    (report.checks as Record<string, unknown>).tenant = "ok";
  } catch (e) {
    report.ok = false;
    (report.checks as Record<string, unknown>).tenant = e instanceof Error ? e.message : "erreur";
  }

  try {
    const tenantSession = await resolveTenantSession();
    (report.checks as Record<string, unknown>).tenantSession = tenantSession ? "ok" : "null";
    if (tenantSession) {
      report.user = { id: tenantSession.userId };
    }

    const user = await resolveTenantCurrentUser();
    (report.checks as Record<string, unknown>).tenantCurrentUser = user ? "ok" : "null";
    if (user) {
      report.user = {
        id: user.id,
        email: user.emailAddresses[0]?.emailAddress ?? null,
        roles: user.publicMetadata?.role ?? null,
      };
    }
  } catch (e) {
    report.ok = false;
    (report.checks as Record<string, unknown>).tenantSession = e instanceof Error ? e.message : "erreur";
  }

  if (isMultiTenantEnabled()) {
    try {
      const session = await resolveSession();
      (report.checks as Record<string, unknown>).nextAuth = session?.userId ? "ok (tenant-session)" : "null";
    } catch (e) {
      (report.checks as Record<string, unknown>).nextAuth =
        e instanceof Error ? e.message : "erreur session tenant";
    }
  }

  const encryptionKeyConfigured = Boolean(process.env.CLERK_ENCRYPTION_KEY?.trim());
  (report.checks as Record<string, unknown>).clerkEncryptionKeyEnv = encryptionKeyConfigured
    ? "present"
    : "missing";

  try {
    const hit = await getJson<{ name?: string }>("settings/site.json");
    (report.checks as Record<string, unknown>).s3Read = hit?.data ? "ok" : "empty_or_missing";
    if (hit?.data && typeof hit.data === "object" && "name" in hit.data) {
      report.siteName = (hit.data as { name?: string }).name ?? null;
    }
  } catch (e) {
    report.ok = false;
    (report.checks as Record<string, unknown>).s3Read = e instanceof Error ? e.message : "erreur";
  }

  try {
    const reservations = await getJson<unknown[]>("reservation-rooms/reservations.json");
    const rooms = await getJson<unknown>("reservation-rooms/rooms.json");
    report.dataSamples = {
      reservationsCount: Array.isArray(reservations?.data) ? reservations.data.length : 0,
      roomsConfigured: Boolean(rooms?.data),
    };
    (report.checks as Record<string, unknown>).reservationRooms = "ok";
  } catch (e) {
    report.ok = false;
    (report.checks as Record<string, unknown>).reservationRooms = e instanceof Error ? e.message : "erreur";
  }

  const checks = report.checks as Record<string, unknown>;
  if (checks.tenantSession !== "ok") {
    report.ok = false;
    report.hint =
      "Session intranet illisible côté serveur. Vérifiez la clé secrète Clerk LP (sk_live_…) dans /platform/setup → secrets tenant.";
  }

  return NextResponse.json(report, { status: report.ok ? 200 : 503 });
}
