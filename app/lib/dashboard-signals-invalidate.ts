import "server-only";

import { valkeyDel, valkeyDeleteByPrefix } from "@/app/lib/valkey";
import {
  valkeyKeyDashboardSignalsAbsenceIndex,
  valkeyPrefixDashboardSignalsCreneaux,
  valkeyPrefixDashboardSignalsEtab,
} from "@/app/lib/valkey-keys";

const NS = "scola";

/** Purge caches dashboard (agrégat + sous-caches) pour un établissement. */
export async function invalidateDashboardSignalsForEstablishment(
  etablissementId: string,
): Promise<void> {
  const etabId = etablissementId.trim();
  if (!etabId) return;

  await Promise.all([
    valkeyDeleteByPrefix(valkeyPrefixDashboardSignalsEtab(etabId)),
    valkeyDeleteByPrefix(valkeyPrefixDashboardSignalsCreneaux(etabId)),
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:vs:${etabId}:`),
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:rh-now:${etabId}:`),
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:stages-pend:${etabId}:`),
    valkeyDeleteByPrefix(`${NS}:dash:sig-src:req-board:${etabId}:`),
    valkeyDel(valkeyKeyDashboardSignalsAbsenceIndex(etabId)),
  ]);
}
