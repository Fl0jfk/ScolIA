import { NextResponse } from "next/server";
import {
  clearRdvEmailGateCookie,
  readRdvEmailGateSession,
} from "@/app/lib/rdv-inscription-email-gate";
import {
  getHomeEtablissementForRdv,
  listChildrenForVerifiedParentEmail,
} from "@/app/lib/rdv-inscription-eleve";
import { maskParentEmails } from "@/app/lib/mask-email";

type Ctx = { params: Promise<{ direction: string }> };

/** Session gate + enfants liés à l’e-mail vérifié. */
export async function GET(_req: Request, ctx: Ctx) {
  try {
    const { direction } = await ctx.params;
    const slug = decodeURIComponent(direction || "").trim().toLowerCase();
    const session = await readRdvEmailGateSession({ directionSlug: slug });
    if (!session) {
      return NextResponse.json({ verified: false });
    }

    const [candidates, homeEtablissement] = await Promise.all([
      listChildrenForVerifiedParentEmail({
        etablissementId: session.etablissementId,
        parentEmail: session.email,
      }),
      getHomeEtablissementForRdv(session.etablissementId),
    ]);

    return NextResponse.json({
      verified: true,
      email: session.email,
      candidates: candidates.map((c) => ({
        ...c,
        parents: maskParentEmails(c.parents || [], session.email),
      })),
      homeEtablissement,
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

/** Déconnexion gate (changer d’e-mail). */
export async function DELETE(_req: Request, ctx: Ctx) {
  const { direction } = await ctx.params;
  void direction;
  await clearRdvEmailGateCookie();
  return NextResponse.json({ ok: true });
}
