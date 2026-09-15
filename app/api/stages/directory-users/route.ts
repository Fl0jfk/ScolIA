import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { listDirectoryMembers } from "@/app/lib/directory-members";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention, canViewReferentConventions } from "@/app/lib/stage-access";

function sortKeyLastName(lastName?: string, firstName?: string, email?: string): string {
  return `${String(lastName || "").trim()} ${String(firstName || "").trim()} ${String(email || "").trim()}`.trim();
}

type DirectoryScope = "referents" | "watchers";

function parseScope(raw: string | null): DirectoryScope {
  return raw === "watchers" ? "watchers" : "referents";
}

function memberMatchesScope(roles: string[], scope: DirectoryScope): boolean {
  if (scope === "referents") {
    return roles.includes("professeur");
  }
  // CPE / restauration (visibilité stages)
  return (
    roles.includes("cpe") ||
    roles.includes("surveillant") ||
    roles.includes("accueil") ||
    roles.includes("administratif")
  );
}

/**
 * Annuaire stages :
 * - `?scope=referents` (défaut) → professeurs (PP / référents)
 * - `?scope=watchers` → CPE, surveillants, accueil, administratif
 */
export async function GET(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles) && !canViewReferentConventions(roles)) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const scope = parseScope(new URL(req.url).searchParams.get("scope"));

    const members = await listDirectoryMembers();
    const users = members
      .filter((m) => m.externalUserId && !m.pending)
      .filter((m) => memberMatchesScope(m.roles, scope))
      .map((m) => ({
        externalUserId: m.externalUserId,
        email: m.email,
        firstName: m.firstName,
        lastName: m.lastName,
        displayName: m.displayName,
        roles: m.roles,
      }))
      .sort((a, b) =>
        sortKeyLastName(a.lastName, a.firstName, a.email).localeCompare(
          sortKeyLastName(b.lastName, b.firstName, b.email),
          "fr",
          { sensitivity: "base" },
        ),
      );

    return NextResponse.json({ users, scope });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
