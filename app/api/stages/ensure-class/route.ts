import { NextResponse } from "next/server";

import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canReviewPreconvention, canViewReferentConventions } from "@/app/lib/stage-access";
import { ensureClassRegisteredForStages } from "@/app/lib/stage-periods-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

/**
 * Enregistre une classe dans la config stages (ex. 5e volontaire hors périodes
 * obligatoires) pour qu'elle apparaisse dans Réglages et Suivi classe.
 */
export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles) && !canViewReferentConventions(roles)) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const body = await req.json();
    const className = String(body.className ?? "").trim();
    if (!className) {
      return NextResponse.json({ error: "Classe manquante." }, { status: 400 });
    }
    const schoolYear = String(body.schoolYear ?? "").trim() || currentStageSchoolYear();
    const result = await ensureClassRegisteredForStages(className, schoolYear);
    return NextResponse.json({
      success: true,
      added: result.added,
      className: result.className,
      schoolYear,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
