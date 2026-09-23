import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  getFamilleThread,
  listFamilleThreadMessages,
  markFamilleThreadRead,
} from "@/app/lib/famille-messaging-db";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";

export async function GET(
  _req: Request,
  ctx: { params: Promise<{ threadId: string }> },
) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const isStaff = roles.some((r) => {
    const x = r.toLowerCase();
    return (
      x.includes("admin") ||
      x.includes("cpe") ||
      x.includes("direction") ||
      x.includes("directeur") ||
      x.includes("administratif") ||
      x.includes("viescolaire") ||
      x.includes("vie_scolaire")
    );
  });
  if (!isStaff) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const { threadId } = await ctx.params;
  const thread = await getFamilleThread(etabId, threadId);
  if (!thread) {
    return NextResponse.json({ error: "Conversation introuvable." }, { status: 404 });
  }
  await markFamilleThreadRead(etabId, threadId, "staff");
  const messages = await listFamilleThreadMessages(etabId, threadId);
  return NextResponse.json({ thread, messages });
}
