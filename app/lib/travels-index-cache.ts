import type { TravelsTrip } from "@/app/lib/travels-types";

/** Ne pas réutiliser un hit Valkey vide : self-heal après déploiement / mauvais tenant. */
export function travelsIndexCacheHitUsable(hit: unknown): hit is TravelsTrip[] {
  return Array.isArray(hit) && hit.length > 0;
}
