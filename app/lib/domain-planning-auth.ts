import { safeCurrentUser } from "@/app/lib/intranet-session";
import "server-only";
import { NextResponse } from "next/server";

import { loadDomains } from "@/app/lib/domain-planning-storage";
import type { DomainPlanningBooking } from "@/app/lib/domain-planning-types";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { canAccessDomainPlanningSettingsFromRoles } from "@/app/lib/intranet-role-utils";
import { requireAuth, isIntranetAdmin, type AuthContext } from "@/app/lib/intranet-auth";

export async function isEvarsCoordinator(userId: string): Promise<boolean> {
  return isDomainCoordinator(userId, "evars");
}

export async function isDomainCoordinator(userId: string, domainId: string): Promise<boolean> {
  if (!userId || !domainId) return false;
  const domains = await loadDomains();
  const domain = domains.find((d) => d.id === domainId);
  return Boolean(domain?.coordinatorClerkUserIds.includes(userId));
}

async function isAnyDomainCoordinator(userId: string): Promise<boolean> {
  if (!userId) return false;
  const domains = await loadDomains();
  return domains.some((d) => d.coordinatorClerkUserIds.includes(userId));
}

async function canAccessDomainPlanningSettings(userId: string): Promise<boolean> {
  if (await isIntranetAdmin()) return true;
  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  if (canAccessDomainPlanningSettingsFromRoles(roles)) return true;
  return isAnyDomainCoordinator(userId);
}

/** Liste Clerk : paramétrage ou affectation de créneaux (responsables de domaine). */
async function canListDomainPlanningClerkUsers(userId: string): Promise<boolean> {
  return canAccessDomainPlanningSettings(userId);
}

export async function requireDomainPlanningClerkUsersList(): Promise<
  { ok: true; ctx: AuthContext } | { ok: false; response: NextResponse }
> {
  const gate = await requireAuth();
  if (!gate.ok) return gate;
  if (!(await canListDomainPlanningClerkUsers(gate.ctx.userId))) {
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Non autorisé.", code: "DOMAIN_PLANNING_CLERK_USERS_DENIED" },
        { status: 403 },
      ),
    };
  }
  return gate;
}

export async function canManageDomainBooking(
  booking: DomainPlanningBooking,
  userId: string,
): Promise<boolean> {
  if (await isIntranetAdmin()) return true;
  if (await isDomainCoordinator(userId, booking.domainId)) return true;
  return booking.userId === userId;
}

export async function requireDomainPlanningSettingsAdmin(): Promise<
  { ok: true; ctx: AuthContext } | { ok: false; response: NextResponse }
> {
  const gate = await requireAuth();
  if (!gate.ok) return gate;
  if (!(await canAccessDomainPlanningSettings(gate.ctx.userId))) {
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Réservé aux responsables de domaine ou administrateurs.", code: "DOMAIN_PLANNING_ADMIN_REQUIRED" },
        { status: 403 },
      ),
    };
  }
  return gate;
}

export async function getDomainPlanningUserDisplay(): Promise<{
  userId: string;
  firstName: string;
  lastName: string;
  email: string;
}> {
  const user = await safeCurrentUser();
  if (!user) throw new Error("Non authentifié");
  return {
    userId: user.id,
    firstName: user.firstName ?? "",
    lastName: (user.lastName ?? "").toUpperCase(),
    email: user.primaryEmailAddress?.emailAddress ?? "",
  };
}
