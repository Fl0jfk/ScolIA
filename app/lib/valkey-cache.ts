import "server-only";

import {
  valkeyCached,
  valkeyDel,
  valkeyDeleteByPrefix,
  valkeyGet,
  valkeySet,
  valkeySetJson,
} from "@/app/lib/valkey";
import {
  VALKEY_TTL,
  valkeyKeyMessagingConversations,
  valkeyKeyMessagingMessages,
  valkeyKeyPasskey,
  valkeyKeyProxyAuth,
  valkeyPrefixElevesDossiers,
} from "@/app/lib/valkey-keys";

/** Présence passkey — L2 Valkey + appelant peut garder L1 mémoire. */
export async function cacheGetPasskeyPresence(
  userId: string,
): Promise<boolean | null> {
  const raw = await valkeyGet(valkeyKeyPasskey(userId));
  if (raw === "1") return true;
  if (raw === "0") return false;
  return null;
}

export async function cacheSetPasskeyPresence(
  userId: string,
  hasPasskey: boolean,
): Promise<void> {
  await valkeySet(
    valkeyKeyPasskey(userId),
    hasPasskey ? "1" : "0",
    VALKEY_TTL.passkeyPresence,
  );
}

export async function cacheInvalidatePasskeyPresence(
  userId?: string,
): Promise<void> {
  if (!userId) return;
  await valkeyDel(valkeyKeyPasskey(userId));
}

export async function cacheGetProxyAuth<T>(
  authUserId: string,
  etablissementId: string | null,
): Promise<T | null> {
  const { valkeyGetJson } = await import("@/app/lib/valkey");
  return valkeyGetJson<T>(valkeyKeyProxyAuth(authUserId, etablissementId));
}

export async function cacheSetProxyAuth(
  authUserId: string,
  etablissementId: string | null,
  value: unknown,
): Promise<void> {
  await valkeySetJson(
    valkeyKeyProxyAuth(authUserId, etablissementId),
    value,
    VALKEY_TTL.proxyAuth,
  );
}

export async function cacheInvalidateProxyAuth(
  authUserId: string,
  etablissementId?: string | null,
): Promise<void> {
  if (etablissementId !== undefined) {
    await valkeyDel(valkeyKeyProxyAuth(authUserId, etablissementId));
    return;
  }
  await valkeyDeleteByPrefix(`scola:proxy:${authUserId}:`);
}

export function messagingConversationsCacheKey(
  etablissementId: string,
  userId: string,
): string {
  return valkeyKeyMessagingConversations(etablissementId, userId);
}

export async function cacheInvalidateMessagingForUsers(
  etablissementId: string,
  userIds: string[],
): Promise<void> {
  const keys = userIds.map((id) =>
    valkeyKeyMessagingConversations(etablissementId, id),
  );
  // Invalide aussi les pages messages de ces users via préfixe conversation
  // (les messages sont keyed par conversationId — invalidés séparément).
  await valkeyDel(...keys);
}

export async function cacheInvalidateMessagingMessages(
  etablissementId: string,
  conversationId: string,
): Promise<void> {
  await valkeyDeleteByPrefix(
    `scola:msg:msgs:${etablissementId}:${conversationId}:`,
  );
}

export async function cacheInvalidateElevesDossiers(
  etablissementId: string,
): Promise<void> {
  await valkeyDeleteByPrefix(valkeyPrefixElevesDossiers(etablissementId));
}

export { valkeyCached, VALKEY_TTL, valkeyKeyMessagingMessages };
