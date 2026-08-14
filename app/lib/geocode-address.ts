type NominatimResult = {
  lat?: string;
  lon?: string;
  display_name?: string;
};

type GeocodedAddress = {
  latitude: number;
  longitude: number;
  displayName?: string;
};

export async function geocodeFrenchAddress(parts: {
  street?: string;
  zip?: string;
  city?: string;
}): Promise<GeocodedAddress | null> {
  const street = String(parts.street || "").trim();
  const zip = String(parts.zip || "").trim();
  const city = String(parts.city || "").trim();
  const query = [street, zip, city].filter(Boolean).join(", ");
  if (!query) return null;

  const url = new URL("https://nominatim.openstreetmap.org/search");
  url.searchParams.set("q", query);
  url.searchParams.set("format", "json");
  url.searchParams.set("limit", "1");
  url.searchParams.set("countrycodes", "fr");

  const res = await fetch(url.toString(), {
    headers: { "User-Agent": "docsLaPro/1.0 (school intranet)" },
    next: { revalidate: 0 },
  });
  if (!res.ok) return null;

  const data = (await res.json()) as NominatimResult[];
  const hit = data[0];
  const latitude = Number(hit?.lat);
  const longitude = Number(hit?.lon);
  if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) return null;

  return {
    latitude,
    longitude,
    displayName: hit?.display_name,
  };
}
