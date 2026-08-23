import { NextResponse } from "next/server";
import { getAppSession } from "@/app/lib/app-session";
import { authProviderStatus } from "@/app/lib/auth-config";
import { isDatabaseConfigured } from "@/db/index";

/** Diagnostic auth Better-Auth (admin / debug). */
export async function GET() {
  const session = await getAppSession();
  const cutover = authProviderStatus();
  return NextResponse.json({
    ...cutover,
    databaseConfigured: isDatabaseConfigured(),
    session: session
      ? {
          userId: session.user.id,
          businessUserId: session.user.businessUserId,
          authSource: session.user.authSource,
          etablissementId: session.user.etablissementId ?? null,
          roles: session.user.roles,
        }
      : null,
  });
}
