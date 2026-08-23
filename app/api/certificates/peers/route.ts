import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { canAccessCertificatesModule } from "@/app/lib/certificates-auth";
import { formatCertificatePersonLabel } from "@/app/lib/certificates-person-label";

function norm(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  if (!canAccessCertificatesModule(user)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  const users = await listDirectoryMembers();
  const peers = users
    .filter((u) => u.externalUserId && !u.pending)
    .map((u) => {
      const firstName = (u.firstName || "").trim();
      const lastName = (u.lastName || "").trim();
      const displayName = u.displayName || `${firstName} ${lastName}`.trim() || u.email;
      return {
        externalUserId: u.externalUserId,
        firstName,
        lastName,
        displayName,
        label: formatCertificatePersonLabel({ firstName, lastName, displayName, email: u.email }),
        email: u.email,
      };
    })
    .sort((a, b) => {
      const byLast = norm(a.lastName).localeCompare(norm(b.lastName), "fr");
      if (byLast !== 0) return byLast;
      const byFirst = norm(a.firstName).localeCompare(norm(b.firstName), "fr");
      if (byFirst !== 0) return byFirst;
      return norm(a.displayName).localeCompare(norm(b.displayName), "fr");
    });

  return NextResponse.json({ peers });
}
