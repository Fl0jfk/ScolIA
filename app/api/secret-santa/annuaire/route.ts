import { NextResponse } from "next/server";
import { requireSantaContext, santaErrorResponse } from "@/app/lib/secret-santa-access";
import { listSantaStaff } from "@/app/lib/secret-santa-db";

export const dynamic = "force-dynamic";

/** Personnel à inviter : noms et rôles seulement, sans adresses e-mail. */
export async function GET() {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth.response;
  try {
    const staff = await listSantaStaff(auth.ctx.etablissementId);
    return NextResponse.json(
      {
        me: auth.ctx.userId,
        staff: staff.map(({ userId, displayName, roles }) => ({ userId, displayName, roles })),
      },
      { headers: { "Cache-Control": "no-store" } },
    );
  } catch (error) {
    return santaErrorResponse(error);
  }
}
