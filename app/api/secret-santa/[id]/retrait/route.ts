import { after, NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { withdrawAfterDraw } from "@/app/lib/secret-santa-db";
import { sendSantaMails } from "@/app/lib/secret-santa-mail";

export const dynamic = "force-dynamic";

const RETRY_DELAYS_MS = [0, 5_000, 20_000];

/**
 * Désistement après le tirage, saisi par l'organisateur. Les avertissements partent après la
 * réponse : ni sa durée ni son contenu ne disent si quelqu'un, ou qui, a été prévenu.
 */
export async function POST(req: Request, { params }: { params: Promise<{ id: string }> }) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    const body = (await req.json().catch(() => null)) as { userId?: unknown } | null;
    const { santa, toNotify } = await withdrawAfterDraw(
      auth.ctx.etablissementId,
      id,
      auth.ctx.userId,
      String(body?.userId ?? ""),
    );
    after(async () => {
      let pending = toNotify;
      for (const delay of RETRY_DELAYS_MS) {
        if (!pending.length) return;
        if (delay) await new Promise((resolve) => setTimeout(resolve, delay));
        const failed: typeof pending = [];
        try {
          await sendSantaMails("changed", santa, pending, async (target) => {
            failed.push(target);
          });
        } catch (error) {
          console.error("[secret-santa] avertissement non envoyé", error instanceof Error ? error.name : "erreur");
          continue;
        }
        pending = failed;
      }
    });
    return NextResponse.json({ success: true });
  } catch (error) {
    return santaErrorResponse(error);
  }
}
