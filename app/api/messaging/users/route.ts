import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { listMessagingUsers } from "@/app/lib/messaging/service";

export async function GET() {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;
  try {
    const users = await listMessagingUsers(gate.ctx.etablissementId, gate.ctx.userId);
    return NextResponse.json({ users });
  } catch (error) {
    console.error("[messaging/users]", error);
    return NextResponse.json(
      { error: "Impossible de lister le personnel." },
      { status: 500 },
    );
  }
}
