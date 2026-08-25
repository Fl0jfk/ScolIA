import { NextResponse } from "next/server";
import { z } from "zod";
import { isDatabaseConfigured } from "@/db/index";
import { isBetterAuthConfigured } from "@/app/lib/auth-config";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  listPasswordActivationTargets,
  sendPasswordActivationBatch,
} from "@/app/lib/password-activation";

const bodySchema = z.object({
  email: z.string().email().optional(),
  dryRun: z.boolean().optional(),
});

/** Envoie un mail d’activation MDP (admin établissement). */
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
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const email = body.email?.trim().toLowerCase();
  const dryRun = body.dryRun ?? false;

  try {
    const targets = await listPasswordActivationTargets(email ? { email } : undefined);

    if (targets.length === 0) {
      return NextResponse.json(
        {
          ok: false,
          error: email
            ? "Utilisateur introuvable ou MFA déjà activée."
            : "Aucun destinataire éligible.",
        },
        { status: 404 },
      );
    }

    if (dryRun) {
      return NextResponse.json({
        ok: true,
        dryRun: true,
        recipients: targets.map((t) => ({
          email: t.email,
          firstName: t.firstName,
          lastName: t.lastName,
        })),
      });
    }

    const batch = await sendPasswordActivationBatch(email ? { email, delayMs: 0 } : undefined);
    const sent = batch.results.filter((r) => r.ok).length;
    const failed = batch.results.filter((r) => !r.ok);

    return NextResponse.json({
      ok: failed.length === 0,
      sent,
      failed: failed.length,
      baseUrl: batch.baseUrl,
      redirectTo: batch.redirectTo,
      results: batch.results,
    });
  } catch (error) {
    console.error("[admin/auth/send-activation]", error);
    return NextResponse.json({ error: "Envoi impossible." }, { status: 500 });
  }
}
