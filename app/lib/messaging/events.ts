import "server-only";

export type MessagingBusEvent = {
  type: string;
  etablissementId: string;
  userIds: string[];
  payload: unknown;
};

type MessagingListener = (event: MessagingBusEvent) => void;

type MessagingBusState = {
  listenersByUser: Map<string, Set<MessagingListener>>;
};

const GLOBAL_KEY = "__scola_messaging_bus__" as const;

function getBusState(): MessagingBusState {
  const g = globalThis as typeof globalThis & {
    [GLOBAL_KEY]?: MessagingBusState;
  };
  if (!g[GLOBAL_KEY]) {
    g[GLOBAL_KEY] = {
      listenersByUser: new Map(),
    };
  }
  return g[GLOBAL_KEY];
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
 * Diffuse un événement aux destinataires listés.
 * Abstraction prête pour LISTEN/NOTIFY Postgres plus tard.
 */
export function publish(event: MessagingBusEvent): void {
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
