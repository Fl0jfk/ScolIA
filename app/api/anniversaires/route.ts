import { NextResponse } from "next/server";
import { getAppSession } from "@/app/lib/app-session";
import { resolveActiveSupervision } from "@/app/lib/supervision";
import { INTRANET_ROLE_OPTIONS } from "@/app/lib/intranet-roles";
import { parisDateKey } from "@/app/lib/paris-time";
import { listAnniversairesDuJour } from "@/app/lib/anniversaires";
import { canSeeEleveAnniversaires } from "@/app/lib/anniversaires-date";
import { requireTenantId } from "@/app/lib/tenant-scope";

export const dynamic = "force-dynamic";

const NON_INTERNES = new Set(["parent", "eleve"]);
const INTERNES = new Set([
  ...INTRANET_ROLE_OPTIONS.map((r) => r.slug).filter((s) => !NON_INTERNES.has(s)),
  "master",
]);

const headers = { "Cache-Control": "no-store" };

export async function GET() {
  try {
    const session = await getAppSession();
    if (!session) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 401, headers });
    }
    const user = session.user;
    const supervision = await resolveActiveSupervision({ actorUserId: user.id });
    if (supervision) {
      return NextResponse.json({ error: "Indisponible en supervision." }, { status: 403, headers });
    }
    if (!user.roles.some((r) => INTERNES.has(r)) || !user.etablissementId) {
      return NextResponse.json({ error: "Accès refusé." }, { status: 403, headers });
    }
    // L'établissement de l'adresse consultée doit être celui du compte (fail-closed).
    const tenant = await requireTenantId();
    if (!tenant.ok) return tenant.response;
    if (tenant.ctx.etablissementId !== user.etablissementId) {
      return NextResponse.json({ error: "Accès refusé." }, { status: 403, headers });
    }
    const todayKey = parisDateKey(new Date());
    const data = await listAnniversairesDuJour(user.etablissementId, todayKey, {
      includeEleves: canSeeEleveAnniversaires(user.roles),
    });
    return NextResponse.json({ todayKey, ...data }, { headers });
  } catch (error) {
    console.error("[api/anniversaires]", error instanceof Error ? error.name : "error");
    return NextResponse.json({ error: "Service indisponible." }, { status: 503, headers });
  }
}
