import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import { provisionRhOnboarding } from "@/app/lib/rh/onboarding-workflow";


/** Valide le dossier : OneDrive + PDF + invitation (pending). */
export async function POST(
  req: Request,
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
    const body = await req.json().catch(() => ({}));
    const validationNote = String((body as { note?: string })?.note || "").trim();
    const record = await provisionRhOnboarding(
      id,
      user.fullName || user.firstName || "RH",
      validationNote,
    );
    return NextResponse.json({ ok: true, record });
  } catch (e) {
    console.error("[rh/onboarding/validate]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Validation impossible." },
      { status: 500 },
    );
  }
}
