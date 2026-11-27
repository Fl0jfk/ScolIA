import "server-only";

import { eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { user, userAvatar } from "@/db/schema";
import { avatarUrl } from "@/app/lib/avatar-rules";

export async function readAvatar(userId: string): Promise<{ image: Buffer; updatedAt: Date } | null> {
  const [row] = await getDb()
    .select({ image: userAvatar.image, updatedAt: userAvatar.updatedAt })
    .from(userAvatar)
    .where(eq(userAvatar.userId, userId));
  return row ?? null;
}

export async function readUserImage(userId: string): Promise<string | undefined> {
  const [row] = await getDb().select({ image: user.image }).from(user).where(eq(user.id, userId));
  return row?.image ?? undefined;
}

/** Dates de mise à jour (sans les images) des personnes qui ont une photo. */
export async function listAvatarVersions(userIds: string[]): Promise<Map<string, Date>> {
  if (!userIds.length) return new Map();
  const rows = await getDb()
    .select({ userId: userAvatar.userId, updatedAt: userAvatar.updatedAt })
    .from(userAvatar)
    .where(inArray(userAvatar.userId, userIds));
  return new Map(rows.map((r) => [r.userId, r.updatedAt]));
}

/** `user.image` pointe vers la photo : menu et messagerie l'affichent sans autre changement. */
export async function saveAvatar(userId: string, image: Buffer, updatedBy: string): Promise<string> {
  const updatedAt = new Date();
  const url = avatarUrl(userId, updatedAt);
  await getDb().transaction(async (tx) => {
    await tx
      .insert(userAvatar)
      .values({ userId, image, updatedBy, updatedAt })
      .onConflictDoUpdate({ target: userAvatar.userId, set: { image, updatedBy, updatedAt } });
    await tx.update(user).set({ image: url, updatedAt }).where(eq(user.id, userId));
  });
  return url;
}

export async function deleteAvatar(userId: string): Promise<void> {
  await getDb().transaction(async (tx) => {
    await tx.delete(userAvatar).where(eq(userAvatar.userId, userId));
    await tx.update(user).set({ image: null, updatedAt: new Date() }).where(eq(user.id, userId));
  });
}

export function avatarResponse(image: Buffer): Response {
  return new Response(new Uint8Array(image), {
    headers: {
      "Content-Type": "image/jpeg",
      "Cache-Control": "private, max-age=3600",
      "X-Content-Type-Options": "nosniff",
    },
  });
}
