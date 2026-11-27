import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { claimInvitations } from "@/app/lib/secret-santa-db";
import { sendSantaInvitations } from "@/app/lib/secret-santa-mail";
import { SANTA_RESEND_MIN_INTERVAL_MS } from "@/app/lib/secret-santa";

export const dynamic = "force-dynamic";

/** Envoi à la demande de l'organisateur : `new` = jamais invités, `unopened` = relance. */
export async function POST(req: Request, { params }: { params: Promise<{ id: string }> }) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    const body = (await req.json().catch(() => null)) as { mode?: unknown } | null;
    const { santa, targets } = await claimInvitations(
      auth.ctx.etablissementId,
      id,
      auth.ctx.userId,
      body?.mode === "unopened" ? "unopened" : "new",
      new Date(),
      SANTA_RESEND_MIN_INTERVAL_MS,
    );
    return NextResponse.json(await sendSantaInvitations(santa, targets));
  } catch (error) {
    return santaErrorResponse(error);
  }
}
