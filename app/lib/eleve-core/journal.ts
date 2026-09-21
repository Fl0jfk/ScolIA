import "server-only";

import { getDb } from "@/db/index";
import { metierEvent } from "@/db/schema";
import type { MetierEventRecord } from "@/app/lib/eleve-core/events";

export async function recordMetierEvent(event: MetierEventRecord): Promise<void> {
  const db = getDb();
  await db.insert(metierEvent).values({
    etablissementId: event.etablissementId,
    type: event.type,
    aggregate: event.aggregate,
    aggregateId: event.aggregateId,
    eleveId: event.eleveId ?? null,
    payload: event.payload ?? {},
    actorUserId: event.actorUserId ?? null,
  });
}
