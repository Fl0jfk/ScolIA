import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { claimInvitations, releaseInvitation, releaseUpdate, type SantaInviteMode } from "@/app/lib/secret-santa-db";
import { sendSantaMails } from "@/app/lib/secret-santa-mail";
import { SANTA_RESEND_MIN_INTERVAL_MS } from "@/app/lib/secret-santa";

export const dynamic = "force-dynamic";

/** Envoi à la demande de l'organisateur : invitation, relance ou informations mises à jour. */
export async function POST(req: Request, { params }: { params: Promise<{ id: string }> }) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    const body = (await req.json().catch(() => null)) as { mode?: unknown } | null;
    const mode: SantaInviteMode = body?.mode === "reminder" || body?.mode === "update" ? body.mode : "new";
    const { santa, targets, previousUpdateAt } = await claimInvitations(
      auth.ctx.etablissementId,
      id,
      auth.ctx.userId,
      mode,
      new Date(),
      SANTA_RESEND_MIN_INTERVAL_MS,
    );
    if (mode !== "update") {
      return NextResponse.json(
        await sendSantaMails("invitation", santa, targets, (t) => releaseInvitation(santa.id, t.userId, t.invitedAt)),
      );
    }
    let result: { sent: number; failed: number };
    try {
      result = await sendSantaMails("update", santa, targets);
    } catch (error) {
      await releaseUpdate(santa.id, previousUpdateAt);
      throw error;
    }
    if (targets.length && result.sent === 0) await releaseUpdate(santa.id, previousUpdateAt);
    return NextResponse.json(result);
  } catch (error) {
    return santaErrorResponse(error);
  }
}
