import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import {
  listFamilleThreadsForFoyers,
  replyFamilleThreadMessage,
} from "@/app/lib/famille-messaging-db";

export async function GET() {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;
  const foyerIds = gate.ctx.foyers.map((f) => f.id);
  const threads = await listFamilleThreadsForFoyers(gate.ctx.etablissementId, foyerIds);
  return NextResponse.json({
    enfants: gate.ctx.enfants,
    foyers: gate.ctx.foyers,
    threads,
  });
}

export async function POST(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;

  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    threadId?: string;
    corps?: string;
  };
  if (body.action !== "reply" || !body.threadId?.trim()) {
    return NextResponse.json({ error: "Action invalide." }, { status: 400 });
  }

  const foyerIds = new Set(gate.ctx.foyers.map((f) => f.id));
  const threads = await listFamilleThreadsForFoyers(gate.ctx.etablissementId, [...foyerIds]);
  const owned = threads.find((t) => t.id === body.threadId.trim());
  if (!owned || !foyerIds.has(owned.foyerId)) {
    return NextResponse.json({ error: "Conversation introuvable." }, { status: 404 });
  }

  const appUser = await requireAppUser();
  const auteurNom = appUser.ok
    ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
      appUser.user.name ||
      appUser.user.email ||
      "Parent"
    : gate.ctx.email;

  try {
    const message = await replyFamilleThreadMessage(gate.ctx.etablissementId, {
      threadId: owned.id,
      auteurCote: "parent",
      auteurUserId: gate.ctx.authUserId,
      auteurNom,
      corps: String(body.corps || ""),
    });
    return NextResponse.json({ success: true, message });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Envoi impossible." },
      { status: 400 },
    );
  }
}
