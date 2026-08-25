import { NextResponse } from "next/server";
import { z } from "zod";
import { isDatabaseConfigured } from "@/db/index";
import { isBetterAuthConfigured, betterAuthBaseUrl } from "@/app/lib/auth-config";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { getTenant } from "@/app/lib/tenant-context";
import {
  resolveInvitationTarget,
  sendPasswordActivationToUser,
} from "@/app/lib/password-activation";

const bodySchema = z.object({
  email: z.string().email(),
});

/**
 * Envoie un lien d’invitation (création / reset MDP) à un utilisateur de l’établissement.
 * Réservé admin établissement.
 */
export async function POST(req: Request) {
  const gate = await requireAdmin();
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

  const email = body.email.trim().toLowerCase();

  try {
    const tenant = await getTenant();
    const etablissementId = await ensureEtablissementFromTenant(tenant);
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

    if (resolved.status === "mfa_already_enabled") {
      return NextResponse.json(
        {
          ok: false,
          error:
            "Ce compte est déjà activé (double authentification en place). La personne peut se connecter normalement, ou utiliser « Mot de passe oublié » si besoin.",
        },
        { status: 409 },
      );
    }

    const result = await sendPasswordActivationToUser(resolved.target);
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

    return NextResponse.json({
      ok: true,
      email: result.email,
      message: `Lien d’invitation envoyé à ${result.email}. Valable 1 heure — la personne crée son mot de passe puis active la MFA.`,
      baseUrl: betterAuthBaseUrl(),
      redirectTo: `${betterAuthBaseUrl()}/auth/reset-password`,
    });
  } catch (error) {
    console.error("[admin/auth/send-activation]", error);
    return NextResponse.json({ error: "Envoi impossible." }, { status: 500 });
  }
}
