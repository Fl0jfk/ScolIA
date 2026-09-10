import "server-only";

import { and, eq, lt } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  messagingAttachment,
  messagingMessage,
  messagingReaction,
} from "@/db/schema";
import { RETENTION_DAYS } from "@/app/lib/messaging/constants";
import { deleteObject } from "@/app/lib/s3-storage";

/**
 * Purge unitaire des messages plus anciens que RETENTION_DAYS (365).
 * Supprime aussi réactions + pièces jointes S3 associées.
 * Jamais de wipe de table entière.
 */
export async function purgeExpiredMessages(
  etabId?: string,
): Promise<{ deletedMessages: number; deletedAttachments: number }> {
  const cutoff = new Date();
  cutoff.setUTCDate(cutoff.getUTCDate() - RETENTION_DAYS);

  const db = getDb();
  const conditions = [lt(messagingMessage.createdAt, cutoff)];
  if (etabId?.trim()) {
    conditions.push(eq(messagingMessage.etablissementId, etabId.trim()));
  }

  const expired = await db
    .select({
      id: messagingMessage.id,
      etablissementId: messagingMessage.etablissementId,
    })
    .from(messagingMessage)
    .where(and(...conditions));

  if (expired.length === 0) {
    return { deletedMessages: 0, deletedAttachments: 0 };
  }

  let deletedAttachments = 0;
  let deletedMessages = 0;

  for (const msg of expired) {
    const attachments = await db
      .select({
        id: messagingAttachment.id,
        s3Key: messagingAttachment.s3Key,
      })
      .from(messagingAttachment)
      .where(
        and(
          eq(messagingAttachment.etablissementId, msg.etablissementId),
          eq(messagingAttachment.messageId, msg.id),
        ),
      );

    for (const att of attachments) {
      try {
        await deleteObject(att.s3Key);
      } catch (error) {
        console.error("[messaging/retention] deleteObject failed", att.s3Key, error);
      }
      await db
        .delete(messagingAttachment)
        .where(
          and(
            eq(messagingAttachment.etablissementId, msg.etablissementId),
            eq(messagingAttachment.id, att.id),
          ),
        );
      deletedAttachments += 1;
    }

    await db
      .delete(messagingReaction)
      .where(
        and(
          eq(messagingReaction.etablissementId, msg.etablissementId),
          eq(messagingReaction.messageId, msg.id),
        ),
      );

    await db
      .delete(messagingMessage)
      .where(
        and(
          eq(messagingMessage.etablissementId, msg.etablissementId),
          eq(messagingMessage.id, msg.id),
        ),
      );
    deletedMessages += 1;
  }

  return { deletedMessages, deletedAttachments };
}
