import { NextResponse } from "next/server";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import {
  countUnreadNotifsForFoyers,
  listFamilleNotifsForFoyers,
} from "@/app/lib/famille-messaging-db";
import { and, eq, inArray, isNull } from "drizzle-orm";
import { getDb } from "@/db/index";
import { familleNotif } from "@/db/schema";

export async function GET(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;
  const foyerIds = gate.ctx.foyers.map((f) => f.id);
  const url = new URL(req.url);
  const unreadOnly = url.searchParams.get("unread") === "1";
  const [notifs, unreadCount] = await Promise.all([
    listFamilleNotifsForFoyers(gate.ctx.etablissementId, foyerIds, {
      unreadOnly,
      limit: 40,
    }),
    countUnreadNotifsForFoyers(gate.ctx.etablissementId, foyerIds),
  ]);
  return NextResponse.json({ notifs, unreadCount });
}

export async function POST(req: Request) {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate.response;
  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    ids?: string[];
  };
  if (body.action !== "mark_read") {
    return NextResponse.json({ error: "Action invalide." }, { status: 400 });
  }
  const foyerIds = gate.ctx.foyers.map((f) => f.id);
  if (!foyerIds.length) return NextResponse.json({ success: true });
  const db = getDb();
  const now = new Date();
  if (Array.isArray(body.ids) && body.ids.length) {
    await db
      .update(familleNotif)
      .set({ luAt: now })
      .where(
        and(
          eq(familleNotif.etablissementId, gate.ctx.etablissementId),
          inArray(familleNotif.foyerId, foyerIds),
          inArray(familleNotif.id, body.ids.map(String)),
          isNull(familleNotif.luAt),
        ),
      );
  } else {
    await db
      .update(familleNotif)
      .set({ luAt: now })
      .where(
        and(
          eq(familleNotif.etablissementId, gate.ctx.etablissementId),
          inArray(familleNotif.foyerId, foyerIds),
          isNull(familleNotif.luAt),
        ),
      );
  }
  return NextResponse.json({ success: true });
}
