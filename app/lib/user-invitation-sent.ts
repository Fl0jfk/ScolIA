import "server-only";

import { sql } from "drizzle-orm";
import { getDb } from "@/db/index";

export { INVITATION_RECENT_MS, invitationRecentlySent } from "@/app/lib/invitation-window";

let ensured = false;

/** Migration légère : colonne invitation_sent_at absente sur certains tenants. */
export async function ensureUserInvitationSentAtColumn(): Promise<void> {
  if (ensured) return;
  const db = getDb();
  await db.execute(
    sql.raw(
      `ALTER TABLE "user" ADD COLUMN IF NOT EXISTS "invitation_sent_at" timestamp with time zone`,
    ),
  );
  ensured = true;
}
