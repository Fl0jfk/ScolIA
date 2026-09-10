import "server-only";

import postgres from "postgres";

export type MessagingBusEvent = {
  type: string;
  etablissementId: string;
  userIds: string[];
  payload: unknown;
};

type MessagingListener = (event: MessagingBusEvent) => void;

type MessagingBusState = {
  listenersByUser: Map<string, Set<MessagingListener>>;
  notifySql: ReturnType<typeof postgres> | null;
  listenSql: ReturnType<typeof postgres> | null;
  listenStarted: boolean;
  instanceId: string;
};

const GLOBAL_KEY = "__scola_messaging_bus_v3__" as const;
const PG_CHANNEL = "scola_messaging";

function getBusState(): MessagingBusState {
  const g = globalThis as typeof globalThis & {
    [GLOBAL_KEY]?: MessagingBusState;
  };
  if (!g[GLOBAL_KEY]) {
    g[GLOBAL_KEY] = {
      listenersByUser: new Map(),
      notifySql: null,
      listenSql: null,
      listenStarted: false,
      instanceId: crypto.randomUUID(),
    };
  }
  return g[GLOBAL_KEY];
}

function deliverLocal(event: MessagingBusEvent): void {
  const state = getBusState();
  const seen = new Set<string>();
  for (const rawId of event.userIds) {
    const uid = rawId.trim();
    if (!uid || seen.has(uid)) continue;
    seen.add(uid);
    const listeners = state.listenersByUser.get(uid);
    if (!listeners) continue;
    for (const listener of listeners) {
      try {
        listener(event);
      } catch (error) {
        console.error("[messaging/events] listener error", error);
      }
    }
  }
}

function isBusEvent(value: unknown): value is MessagingBusEvent {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.type === "string" &&
    typeof v.etablissementId === "string" &&
    Array.isArray(v.userIds)
  );
}

async function ensurePgListen(): Promise<void> {
  const state = getBusState();
  if (state.listenStarted) return;
  const url = process.env.DATABASE_URL?.trim();
  if (!url) return;
  state.listenStarted = true;
  try {
    state.listenSql = postgres(url, {
      max: 1,
      prepare: false,
      idle_timeout: 0,
      max_lifetime: 0,
    });
    await state.listenSql.listen(PG_CHANNEL, (raw) => {
      void (async () => {
        try {
          const envelope = JSON.parse(raw) as {
            origin?: string;
            relayId?: string;
            event?: MessagingBusEvent;
          };
          if (envelope.origin && envelope.origin === state.instanceId) return;

          let event: MessagingBusEvent | null = null;
          if (envelope.relayId && state.listenSql) {
            const rows = await state.listenSql<
              { event: MessagingBusEvent }[]
            >`SELECT event FROM messaging_signal_relay WHERE id = ${envelope.relayId} LIMIT 1`;
            event = rows[0]?.event ?? null;
            await state.listenSql`
              DELETE FROM messaging_signal_relay WHERE id = ${envelope.relayId}
            `.catch(() => undefined);
          } else if (envelope.event && isBusEvent(envelope.event)) {
            event = envelope.event;
          } else if (isBusEvent(envelope)) {
            event = envelope;
          }
          if (event) deliverLocal(event);
        } catch (error) {
          console.error("[messaging/events] listen handle error", error);
        }
      })();
    });
  } catch (error) {
    state.listenStarted = false;
    console.error("[messaging/events] LISTEN failed", error);
  }
}

async function notifyRemote(event: MessagingBusEvent): Promise<void> {
  const url = process.env.DATABASE_URL?.trim();
  if (!url) return;
  const state = getBusState();
  try {
    if (!state.notifySql) {
      state.notifySql = postgres(url, { max: 1, prepare: false });
    }
    const origin = state.instanceId;
    const full = JSON.stringify({ origin, event });
    if (full.length <= 7500) {
      await state.notifySql.notify(PG_CHANNEL, full);
      return;
    }
    const relayId = crypto.randomUUID();
    await state.notifySql`
      INSERT INTO messaging_signal_relay (id, event)
      VALUES (${relayId}, ${state.notifySql.json(event as never)})
    `;
    await state.notifySql.notify(
      PG_CHANNEL,
      JSON.stringify({ origin, relayId }),
    );
  } catch (error) {
    console.error("[messaging/events] NOTIFY failed", error);
  }
}

/** Abonne un utilisateur aux événements messagerie (HMR-safe via globalThis). */
export function subscribe(userId: string, listener: MessagingListener): () => void {
  const uid = userId.trim();
  if (!uid) return () => undefined;

  const state = getBusState();
  let set = state.listenersByUser.get(uid);
  if (!set) {
    set = new Set();
    state.listenersByUser.set(uid, set);
  }
  set.add(listener);
  void ensurePgListen();

  return () => {
    const current = state.listenersByUser.get(uid);
    if (!current) return;
    current.delete(listener);
    if (current.size === 0) {
      state.listenersByUser.delete(uid);
    }
  };
}

/**
 * Diffuse un événement aux destinataires (local + autres instances via Postgres NOTIFY).
 */
export function publish(event: MessagingBusEvent): void {
  deliverLocal(event);
  void notifyRemote(event);
}
