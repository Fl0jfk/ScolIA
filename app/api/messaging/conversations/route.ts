import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import {
  MessagingError,
  createGroupConversation,
  getOrCreateConversation,
  listConversationsForUser,
} from "@/app/lib/messaging/service";

export async function GET() {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;

  try {
    const conversations = await listConversationsForUser(
      gate.ctx.etablissementId,
      gate.ctx.userId,
    );
    return NextResponse.json({ conversations });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/conversations GET]", error);
    return NextResponse.json({ error: "Erreur serveur." }, { status: 500 });
  }
}

export async function POST(req: Request) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;

  try {
    const body = (await req.json()) as {
      peerUserId?: unknown;
      kind?: unknown;
      title?: unknown;
      memberIds?: unknown;
    };

    if (body.kind === "group") {
      const title = typeof body.title === "string" ? body.title : "";
      const memberIds = Array.isArray(body.memberIds)
        ? body.memberIds.filter((id): id is string => typeof id === "string")
        : [];
      const conversation = await createGroupConversation(
        gate.ctx.etablissementId,
        gate.ctx.userId,
        title,
        memberIds,
      );
      return NextResponse.json({ conversation });
    }

    const peerUserId = typeof body.peerUserId === "string" ? body.peerUserId.trim() : "";
    if (!peerUserId) {
      return NextResponse.json(
        { error: "peerUserId requis.", code: "PEER_REQUIRED" },
        { status: 400 },
      );
    }
    const conversation = await getOrCreateConversation(
      gate.ctx.etablissementId,
      gate.ctx.userId,
      peerUserId,
    );
    return NextResponse.json({ conversation });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/conversations POST]", error);
    return NextResponse.json({ error: "Erreur serveur." }, { status: 500 });
  }
}
