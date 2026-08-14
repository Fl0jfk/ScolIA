import { normalizePublicImageUrl, scolaImageUrl } from "@/app/lib/scola-image";

/** Réécrit les URLs d'illustration voyages vers le CDN public actuel. */
export function normalizeTravelImageUrl(url: string | undefined | null): string | undefined {
  const trimmed = String(url || "").trim();
  if (!trimmed) return undefined;
  return normalizePublicImageUrl(trimmed);
}

export function normalizeTripImageFields<T extends { imageUrl?: string; data?: object }>(
  trip: T,
): T {
  const data = trip.data as { imageUrl?: string } | undefined;
  const top = normalizeTravelImageUrl(trip.imageUrl);
  const nested = data?.imageUrl ? normalizeTravelImageUrl(data.imageUrl) : undefined;

  if (top === trip.imageUrl && nested === data?.imageUrl) return trip;

  return {
    ...trip,
    ...(top !== undefined ? { imageUrl: top } : {}),
    ...(trip.data
      ? {
          data: {
            ...trip.data,
            ...(nested !== undefined ? { imageUrl: nested } : {}),
          },
        }
      : {}),
  };
}

/** @deprecated Utiliser normalizePublicImageUrl / scolaImageUrl. */
function travelImageUrl(path: string): string {
  return scolaImageUrl(path);
}
