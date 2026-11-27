import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { markSantaOpened } from "@/app/lib/secret-santa-db";

export const dynamic = "force-dynamic";

/** `seen` : le nom affiché à l'écran, pour détecter une page restée ouverte pendant un changement. */
export async function POST(req: Request, { params }: { params: Promise<{ id: string }> }) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const { id } = await params;
    const body = (await req.json().catch(() => null)) as { seen?: unknown } | null;
    const stale = await markSantaOpened(
      auth.ctx.etablissementId,
      id,
      auth.ctx.userId,
      typeof body?.seen === "string" ? body.seen : "",
    );
    return NextResponse.json({ success: true, stale });
  } catch (error) {
    return santaErrorResponse(error);
  }
}
