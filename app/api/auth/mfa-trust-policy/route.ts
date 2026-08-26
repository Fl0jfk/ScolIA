import { NextResponse } from "next/server";
import { getBetterAuth } from "@/app/lib/auth-server";
import { resolveMfaTrustPolicyForUserId } from "@/app/lib/mfa-trust-resolve";
import { MFA_TRUST_STAFF_SECONDS, resolveMfaTrustPolicy } from "@/app/lib/two-factor-policy";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user } from "@/db/schema";
import { eq } from "drizzle-orm";

/**
 * Politique « appareil de confiance » pour l’écran MFA.
 * Utilise la session si déjà établie ; sinon le cookie pending 2FA (via session challenge).
 */
export async function GET(req: Request) {
  if (!isDatabaseConfigured()) {
    return NextResponse.json({
      allowTrust: true,
      maxAgeSeconds: MFA_TRUST_STAFF_SECONDS,
      label: "Se souvenir de cet appareil 30 jours",
      hint: "Sur cet appareil, la MFA ne sera pas redemandée pendant 30 jours.",
    });
  }

  try {
    const session = await getBetterAuth().api.getSession({ headers: req.headers });
    const sessionUserId = session?.user?.id;
    if (sessionUserId) {
      const policy = await resolveMfaTrustPolicyForUserId(sessionUserId);
      if (policy) return NextResponse.json(policy);
    }
  } catch {
    /* pending 2FA : pas encore de session */
  }

  // Fallback : e-mail passé juste après sign-in (stocké côté client), non sensible en soi
  // si l’utilisateur vient de s’authentifier (mot de passe OK).
  const email = new URL(req.url).searchParams.get("email")?.trim().toLowerCase();
  if (email) {
    const db = getDb();
    const [row] = await db
      .select({
        id: user.id,
        etablissementId: user.etablissementId,
        platformAdmin: user.platformAdmin,
      })
      .from(user)
      .where(eq(user.email, email))
      .limit(1);
    if (row) {
      const roles = await listUserRolesFromDb(row.id, row.etablissementId);
      return NextResponse.json(
        resolveMfaTrustPolicy({ platformAdmin: row.platformAdmin, roles }),
      );
    }
  }

  return NextResponse.json({
    allowTrust: true,
    maxAgeSeconds: MFA_TRUST_STAFF_SECONDS,
    label: "Se souvenir de cet appareil 30 jours",
    hint: "Sur cet appareil, la MFA ne sera pas redemandée pendant 30 jours.",
  });
}
