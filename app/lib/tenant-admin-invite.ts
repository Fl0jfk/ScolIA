import "server-only";

import { createClerkClient } from "@clerk/backend";
import { hasGlobalAdminRole } from "@/app/lib/intranet-roles";

export type TenantAdminInviteContact = {
  firstName: string;
  lastName: string;
  email: string;
};

/**
 * Invite (ou promeut) l'administrateur global d'un tenant sur son app Clerk.
 * Seul ce mail peut accepter l'invitation et devenir org admin.
 */
export async function inviteAdminOnTenantClerk(
  clerkSecretKey: string,
  admin: TenantAdminInviteContact,
): Promise<void> {
  const client = createClerkClient({ secretKey: clerkSecretKey });
  const email = admin.email.trim().toLowerCase();
  if (!email) throw new Error("E-mail administrateur requis.");
  const firstName = admin.firstName.trim();
  const lastName = admin.lastName.trim();
  const roles = ["admin"];
  const existing = await client.users.getUserList({ emailAddress: [email], limit: 1 });
  const user = existing.data?.[0];
  if (user) {
    await client.users.updateUser(user.id, {
      ...(firstName ? { firstName } : {}),
      ...(lastName ? { lastName } : {}),
      publicMetadata: {
        ...(user.publicMetadata as object),
        role: roles,
        org_admin: hasGlobalAdminRole(roles),
      },
    });
    return;
  }
  await client.invitations.createInvitation({
    emailAddress: email,
    publicMetadata: { role: roles, org_admin: true },
  });
}

export function parseAdminContactFromBody(raw: unknown): TenantAdminInviteContact | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const email = typeof o.email === "string" ? o.email.trim().toLowerCase() : "";
  if (!email || !email.includes("@")) return null;
  return {
    firstName: typeof o.firstName === "string" ? o.firstName.trim() : "",
    lastName: typeof o.lastName === "string" ? o.lastName.trim() : "",
    email,
  };
}
