import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  attachResponsableToMatchedUser,
  backfillResponsableIdentityLinks,
  listMultiMembershipUsers,
  listOrphanResponsablesForEtablissement,
} from "@/app/lib/platform-identity";

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const [orphans, multiMembership] = await Promise.all([
    listOrphanResponsablesForEtablissement(etabId),
    listMultiMembershipUsers(40),
  ]);

  const matchable = orphans.filter((o) => o.matchUserId).length;
  const nameAlerts = orphans.filter((o) => o.matchUserId && o.nameAlert).length;

  return NextResponse.json({
    summary: {
      orphanResponsables: orphans.length,
      matchableByEmail: matchable,
      nameAlerts,
      multiMembershipUsers: multiMembership.length,
    },
    orphans,
    multiMembership,
  });
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "");

  try {
    if (action === "attachOne") {
      const res = await attachResponsableToMatchedUser(etabId, String(body.responsableId || ""));
      if (!res.ok) return NextResponse.json({ error: res.error }, { status: 400 });
      return NextResponse.json({
        ok: true,
        userId: res.userId,
        nameAlert: res.nameAlert,
      });
    }
    if (action === "backfill") {
      const stats = await backfillResponsableIdentityLinks(etabId);
      return NextResponse.json({ ok: true, ...stats });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur identité." },
      { status: 400 },
    );
  }
}
