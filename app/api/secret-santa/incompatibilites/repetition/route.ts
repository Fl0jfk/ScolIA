import { randomInt } from "node:crypto";
import { NextResponse } from "next/server";
import { requireSantaRulesAdmin, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { listSantaStaff } from "@/app/lib/secret-santa-db";
import { listIncompatibilities } from "@/app/lib/secret-santa-rules-db";
import { rehearseDraw } from "@/app/lib/secret-santa-repetition";
import { SantaError } from "@/app/lib/secret-santa";

export const dynamic = "force-dynamic";

/** Faux tirage réservé à l'administrateur : n'ouvre aucun Secret Santa et n'écrit rien. */
export async function POST(req: Request) {
  try {
    const auth = await requireSantaRulesAdmin();
    if (!auth.ok) return auth.response;
    const body = (await req.json().catch(() => null)) as { participantIds?: unknown } | null;
    const asked = Array.isArray(body?.participantIds) ? body.participantIds : [];
    const staff = new Set((await listSantaStaff(auth.ctx.etablissementId)).map((m) => m.userId));
    if (!asked.every((id): id is string => typeof id === "string" && staff.has(id))) {
      throw new SantaError("Une des personnes choisies n'est plus dans l'annuaire du personnel.");
    }
    const result = rehearseDraw(asked, await listIncompatibilities(auth.ctx.etablissementId), (max) => randomInt(max));
    return NextResponse.json(result, { headers: { "Cache-Control": "no-store" } });
  } catch (error) {
    return santaErrorResponse(error);
  }
}
