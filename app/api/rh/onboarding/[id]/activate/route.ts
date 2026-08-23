import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import { activateRhOnboarding } from "@/app/lib/rh/onboarding-workflow";


/** Active le compte (invitation acceptée → statut active dans meta-rh). */
export async function POST(
  _req: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la RH." }, { status: 403 });
  }

  const { id } = await ctx.params;
  try {
    const record = await activateRhOnboarding(id);
    return NextResponse.json({ ok: true, record });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Activation impossible." },
      { status: 400 },
    );
  }
}
