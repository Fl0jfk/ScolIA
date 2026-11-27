import { NextResponse } from "next/server";
import { requireSantaRulesAdmin, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { listOrganizedLists, listSantaStaff } from "@/app/lib/secret-santa-db";
import { addIncompatibility, listIncompatibilities, removeIncompatibility } from "@/app/lib/secret-santa-rules-db";
import { SantaError } from "@/app/lib/secret-santa";

export const dynamic = "force-dynamic";

const NO_STORE = { "Cache-Control": "no-store" };

async function readPair(req: Request): Promise<[string, string]> {
  const body = (await req.json().catch(() => null)) as { a?: unknown; b?: unknown } | null;
  if (typeof body?.a !== "string" || typeof body.b !== "string" || !body.a || !body.b) {
    throw new SantaError("Choisissez deux personnes.");
  }
  return [body.a, body.b];
}

/** Réservé à l'administrateur de l'établissement. */
export async function GET() {
  try {
    const auth = await requireSantaRulesAdmin();
    if (!auth.ok) return auth.response;
    const staff = await listSantaStaff(auth.ctx.etablissementId);
    return NextResponse.json(
      {
        staff: staff.map(({ userId, displayName }) => ({ userId, displayName })),
        rules: await listIncompatibilities(auth.ctx.etablissementId),
        lists: await listOrganizedLists(auth.ctx.etablissementId, auth.ctx.userId),
      },
      { headers: NO_STORE },
    );
  } catch (error) {
    return santaErrorResponse(error);
  }
}

export async function POST(req: Request) {
  try {
    const auth = await requireSantaRulesAdmin();
    if (!auth.ok) return auth.response;
    const [a, b] = await readPair(req);
    const staff = new Set((await listSantaStaff(auth.ctx.etablissementId)).map((m) => m.userId));
    if (!staff.has(a) || !staff.has(b)) throw new SantaError("Personne introuvable parmi le personnel.", 404);
    await addIncompatibility(auth.ctx.etablissementId, a, b, auth.ctx.userId);
    return NextResponse.json({ success: true });
  } catch (error) {
    return santaErrorResponse(error);
  }
}

export async function DELETE(req: Request) {
  try {
    const auth = await requireSantaRulesAdmin();
    if (!auth.ok) return auth.response;
    const [a, b] = await readPair(req);
    await removeIncompatibility(auth.ctx.etablissementId, a, b);
    return NextResponse.json({ success: true });
  } catch (error) {
    return santaErrorResponse(error);
  }
}
