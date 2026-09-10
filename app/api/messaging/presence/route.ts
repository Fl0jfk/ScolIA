import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import {
  getMyPresence,
  getPresenceMap,
  isManualPresenceStatus,
  isPresenceStatus,
  setManualPresence,
  setPresenceStatus,
} from "@/app/lib/messaging/presence";

export const dynamic = "force-dynamic";

const DURATION_HOURS = new Set([0, 1, 4, 24]);

/** GET ?userIds=a,b,c — statuts. GET ?me=1 — mon statut + manuel. */
export async function GET(request: Request) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;

  try {
    const url = new URL(request.url);
    if (url.searchParams.get("me") === "1") {
      const me = await getMyPresence(gate.ctx.etablissementId, gate.ctx.userId);
      return NextResponse.json({ me });
    }

    const raw = url.searchParams.get("userIds") ?? "";
    const userIds = raw
      .split(",")
      .map((id) => id.trim())
      .filter(Boolean)
      .slice(0, 200);

    const presence = await getPresenceMap(gate.ctx.etablissementId, userIds);
    return NextResponse.json({ presence });
  } catch (error) {
    console.error("[messaging/presence] GET failed", error);
    return NextResponse.json(
      {
        error: "Erreur présence.",
        detail: error instanceof Error ? error.message : String(error),
      },
      { status: 500 },
    );
  }
}

/**
 * POST { status, mode?: 'auto'|'manual', durationHours?: 0|1|4|24 }
 * - auto (défaut) : sync onglet / appel
 * - manual : Occupé / NPD / Absent pour une durée
 */
export async function POST(request: Request) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  try {
    const record = body && typeof body === "object" ? (body as Record<string, unknown>) : {};
    const status = record.status;
    const mode = record.mode === "manual" ? "manual" : "auto";

    if (!isPresenceStatus(status) || status === "offline") {
      return NextResponse.json(
        { error: "Statut invalide (online | away | busy | dnd)." },
        { status: 400 },
      );
    }

    if (mode === "manual") {
      if (!isManualPresenceStatus(status)) {
        return NextResponse.json({ error: "Statut manuel invalide." }, { status: 400 });
      }
      const durationRaw = record.durationHours;
      const durationHours =
        typeof durationRaw === "number" && DURATION_HOURS.has(durationRaw) ? durationRaw : 24;
      const me = await setManualPresence(
        gate.ctx.etablissementId,
        gate.ctx.userId,
        status,
        durationHours,
      );
      return NextResponse.json({ me, status: me.status });
    }

    const next = await setPresenceStatus(
      gate.ctx.etablissementId,
      gate.ctx.userId,
      status,
    );
    return NextResponse.json({ status: next });
  } catch (error) {
    console.error("[messaging/presence] POST failed", error);
    return NextResponse.json(
      {
        error: "Erreur présence.",
        detail: error instanceof Error ? error.message : String(error),
      },
      { status: 500 },
    );
  }
}
