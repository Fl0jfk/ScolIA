import { requireMessagingContext } from "@/app/lib/messaging/access";
import { subscribe, type MessagingBusEvent } from "@/app/lib/messaging/events";
import {
  heartbeatPresence,
  touchPresenceConnection,
} from "@/app/lib/messaging/presence";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

function sseEncode(data: unknown): string {
  return `data: ${JSON.stringify(data)}\n\n`;
}

export async function GET() {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;

  const userId = gate.ctx.userId;
  const etablissementId = gate.ctx.etablissementId;
  const encoder = new TextEncoder();
  let unsubscribe: (() => void) | null = null;
  let heartbeat: ReturnType<typeof setInterval> | null = null;
  let closed = false;
  let presenceOpen: Promise<void> | null = null;

  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      const send = (payload: unknown) => {
        if (closed) return;
        try {
          controller.enqueue(encoder.encode(sseEncode(payload)));
        } catch {
          closed = true;
        }
      };

      send({ type: "heartbeat" });

      presenceOpen = touchPresenceConnection(etablissementId, userId, 1)
        .then(() => undefined)
        .catch((error) => {
          console.error("[messaging/stream] presence open failed", error);
        });

      unsubscribe = subscribe(userId, (event: MessagingBusEvent) => {
        send({
          type: event.type,
          conversationId:
            typeof event.payload === "object" &&
            event.payload &&
            "conversationId" in event.payload
              ? String((event.payload as { conversationId?: string }).conversationId ?? "")
              : undefined,
          payload: event.payload,
        });
      });

      heartbeat = setInterval(() => {
        send({ type: "heartbeat" });
        void heartbeatPresence(etablissementId, userId).catch(() => undefined);
      }, 25_000);
    },
    cancel() {
      closed = true;
      if (heartbeat) clearInterval(heartbeat);
      unsubscribe?.();
      void (async () => {
        await presenceOpen;
        try {
          await touchPresenceConnection(etablissementId, userId, -1);
        } catch (error) {
          console.error("[messaging/stream] presence close failed", error);
        }
      })();
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream; charset=utf-8",
      "Cache-Control": "no-cache, no-transform",
      Connection: "keep-alive",
      "X-Accel-Buffering": "no",
    },
  });
}
