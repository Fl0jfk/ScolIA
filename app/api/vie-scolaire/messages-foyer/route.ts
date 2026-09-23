import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  createFamilleThreadWithMessage,
  listFoyersLight,
  listStaffFamilleThreads,
  replyFamilleThreadMessage,
} from "@/app/lib/famille-messaging-db";

function canStaffMessageFamille(roles: string[]): boolean {
  const set = new Set(roles.map((r) => r.toLowerCase()));
  return (
    set.has("admin") ||
    set.has("orgadmin") ||
    set.has("cpe") ||
    set.has("vie_scolaire") ||
    set.has("viescolaire") ||
    set.has("direction") ||
    set.has("directeur") ||
    set.has("directrice") ||
    set.has("administratif") ||
    set.has("secretariat") ||
    set.has("secrétariat")
  );
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canStaffMessageFamille(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const [threads, foyers] = await Promise.all([
    listStaffFamilleThreads(etabId),
    listFoyersLight(etabId),
  ]);
  return NextResponse.json({ threads, foyers });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canStaffMessageFamille(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    foyerId?: string;
    eleveId?: string;
    sujet?: string;
    corps?: string;
    threadId?: string;
  };

  const auteurNom =
    user?.fullName ||
    [user?.firstName, user?.lastName].filter(Boolean).join(" ") ||
    user?.primaryEmailAddress?.emailAddress ||
    "Établissement";

  try {
    if (body.action === "reply") {
      const message = await replyFamilleThreadMessage(etabId, {
        threadId: String(body.threadId || ""),
        auteurCote: "staff",
        auteurUserId: userId,
        auteurNom,
        corps: String(body.corps || ""),
      });
      return NextResponse.json({ success: true, message });
    }

    const result = await createFamilleThreadWithMessage(etabId, {
      foyerId: String(body.foyerId || ""),
      eleveId: body.eleveId || null,
      sujet: String(body.sujet || ""),
      corps: String(body.corps || ""),
      auteurUserId: userId,
      auteurNom,
    });
    return NextResponse.json({ success: true, ...result });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Envoi impossible." },
      { status: 400 },
    );
  }
}
