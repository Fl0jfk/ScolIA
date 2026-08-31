import { NextResponse } from "next/server";
import { z } from "zod";
import { isDatabaseConfigured } from "@/db/index";
import { isBetterAuthConfigured, betterAuthBaseUrl } from "@/app/lib/auth-config";
import { requireTenantAdminRole } from "@/app/lib/intranet-auth";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { getTenant } from "@/app/lib/tenant-context";
import { getDb } from "@/db/index";
import { user as userTable } from "@/db/schema";
import { eq } from "drizzle-orm";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { roleRequiresTwoFactor } from "@/app/lib/two-factor-policy";
import {
  listPendingInvitationTargets,
  resolveInvitationTarget,
  sendPasswordActivationToUser,
} from "@/app/lib/password-activation";

/** Envoi groupé possible (centaines de mails). */
export const maxDuration = 300;

const bodySchema = z.object({
  email: z.string().email().optional(),
  /** Envoie à tous les comptes encore en attente d’activation n’ayant pas reçu d’invitation récente. */
  bulkPending: z.boolean().optional(),
});

/**
 * Envoie un lien d’invitation (reset MDP, et MFA si déjà active) à un utilisateur
 * de l’établissement. Réservé admin établissement.
 */
export async function POST(req: Request) {
  const gate = await requireTenantAdminRole();
  if (!gate.ok) return gate.response;

  if (!isDatabaseConfigured() || !isBetterAuthConfigured()) {
    return NextResponse.json({ error: "Auth indisponible." }, { status: 503 });
  }

  let body: z.infer<typeof bodySchema>;
  try {
    body = bodySchema.parse(await req.json());
  } catch {
    return NextResponse.json({ error: "Indiquez une adresse e-mail valide." }, { status: 400 });
  }

  try {
    const tenant = await getTenant();
    const etablissementId = await ensureEtablissementFromTenant(tenant);

    if (body.bulkPending) {
      const targets = await listPendingInvitationTargets({ etablissementId });
      if (targets.length === 0) {
        return NextResponse.json({
          ok: true,
          bulk: true,
          sent: 0,
          failed: 0,
          message:
            "Aucun destinataire : tout le monde a déjà activé son compte, ou une invitation récente a déjà été envoyée.",
        });
      }

      let sent = 0;
      let failed = 0;
      const errors: string[] = [];
      for (const target of targets) {
        const result = await sendPasswordActivationToUser(target, { resetMfa: false });
        if (result.ok) sent += 1;
        else {
          failed += 1;
          errors.push(`${target.email}: ${result.detail || result.skipped || "échec"}`);
        }
        await new Promise((r) => setTimeout(r, 350));
      }

      return NextResponse.json({
        ok: failed === 0,
        bulk: true,
        sent,
        failed,
        total: targets.length,
        errors: errors.slice(0, 10),
        message:
          failed === 0
            ? `${sent} invitation(s) envoyée(s). Lien valable 24 heures.`
            : `${sent} envoyée(s), ${failed} échec(s).`,
        baseUrl: betterAuthBaseUrl(),
      });
    }

    const email = body.email?.trim().toLowerCase();
    if (!email) {
      return NextResponse.json({ error: "Indiquez une adresse e-mail valide." }, { status: 400 });
    }

    const resolved = await resolveInvitationTarget({ email, etablissementId });

    if (resolved.status === "not_found") {
      return NextResponse.json(
        {
          ok: false,
          error:
            "Aucun compte trouvé avec cet e-mail dans votre établissement. Créez d’abord l’utilisateur, puis renvoyez le lien d’invitation.",
        },
        { status: 404 },
      );
    }

    const hadMfa = resolved.target.twoFactorEnabled;
    const [userRow] = await getDb()
      .select({ platformAdmin: userTable.platformAdmin, orgAdmin: userTable.orgAdmin })
      .from(userTable)
      .where(eq(userTable.id, resolved.target.id))
      .limit(1);
    const roles = await listUserRolesFromDb(resolved.target.id, etablissementId);
    const mfaRequired = roleRequiresTwoFactor({
      platformAdmin: Boolean(userRow?.platformAdmin),
      orgAdmin: Boolean(userRow?.orgAdmin) || roles.includes("admin"),
      roles,
    });
    /** Prof / surveillant / CPE : on conserve une MFA déjà activée. */
    const resetMfa = hadMfa && mfaRequired;
    const result = await sendPasswordActivationToUser(resolved.target, { resetMfa });
    if (!result.ok) {
      return NextResponse.json(
        {
          ok: false,
          error: result.detail || "Envoi du lien d’invitation impossible.",
          skipped: result.skipped,
        },
        { status: result.skipped === "smtp_unavailable" ? 503 : 500 },
      );
    }

    const message = hadMfa
      ? resetMfa
        ? `Lien d’invitation envoyé à ${result.email}. L’ancien mot de passe et la MFA ont été réinitialisés — la personne repart de zéro (nouveau MDP puis nouvelle MFA). Lien valable 24 heures.`
        : `Lien d’invitation envoyé à ${result.email}. Nouveau mot de passe à créer via le lien (24 h). La double authentification déjà en place est conservée.`
      : mfaRequired
        ? `Lien d’invitation envoyé à ${result.email}. Valable 24 heures — mot de passe puis double authentification obligatoires (direction / personnel administratif).`
        : `Lien d’invitation envoyé à ${result.email}. Valable 24 heures — connexion ensuite avec e-mail et mot de passe (sans double authentification obligatoire).`;

    return NextResponse.json({
      ok: true,
      email: result.email,
      resetMfa: result.resetMfa === true,
      message,
      baseUrl: betterAuthBaseUrl(),
      redirectTo: `${betterAuthBaseUrl()}/auth/reset-password`,
    });
  } catch (error) {
    console.error("[admin/auth/send-activation]", error);
    return NextResponse.json({ error: "Envoi impossible." }, { status: 500 });
  }
}
