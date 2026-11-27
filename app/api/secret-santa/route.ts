import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { createSanta, listSantasFor } from "@/app/lib/secret-santa-db";
import { parseSantaInput } from "@/app/lib/secret-santa";

export const dynamic = "force-dynamic";

export async function GET() {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const santas = await listSantasFor(auth.ctx.etablissementId, auth.ctx.userId);
    return NextResponse.json(
      { santas, serverNow: new Date().toISOString() },
      { headers: { "Cache-Control": "no-store" } },
    );
  } catch (error) {
    return santaErrorResponse(error);
  }
}

export async function POST(req: Request) {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const input = parseSantaInput(await req.json().catch(() => null), new Date());
    const id = await createSanta(
      auth.ctx.etablissementId,
      { userId: auth.ctx.userId, name: auth.ctx.name },
      input,
    );
    return NextResponse.json({ id }, { status: 201 });
  } catch (error) {
    return santaErrorResponse(error);
  }
}
