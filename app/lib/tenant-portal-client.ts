import {
  isBrowserLocalDev,
  localDevSignInUrl,
} from "@/app/lib/local-dev";

const STORAGE_KEY = "scola.portal.lastTenant";

/** URL sign-in fiable : en local on reste sur localhost avec le bon tenant. */
export function catalogEntrySignInUrl(entry: {
  slug?: string;
  signInUrl: string;
  primaryHostname?: string | null;
  appUrl?: string;
}): string {
  if (isBrowserLocalDev()) {
    if (entry.signInUrl.startsWith("/")) {
      const url = new URL(entry.signInUrl, window.location.origin);
      return url.toString();
    }
    return localDevSignInUrl(window.location.origin, entry.slug);
  }

  const host = entry.primaryHostname?.trim().replace(/^https?:\/\//, "").replace(/\/$/, "");
  if (host && host !== "localhost" && host !== "127.0.0.1") {
    return `https://${host}/auth/sign-in`;
  }

  const appUrl = entry.appUrl?.trim().replace(/\/$/, "");
  if (appUrl) {
    try {
      const origin = appUrl.startsWith("http") ? appUrl : `https://${appUrl}`;
      return `${new URL(origin).origin}/auth/sign-in`;
    } catch {
      /* fall through */
    }
  }

  return entry.signInUrl;
}

type SavedPortalTenant = {
  slug: string;
  label: string;
  signInUrl: string;
  savedAt: number;
};

export function saveLastPortalTenant(entry: {
  slug: string;
  label: string;
  signInUrl: string;
}): void {
  if (typeof window === "undefined") return;
  try {
    const payload: SavedPortalTenant = { ...entry, savedAt: Date.now() };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  } catch {
    /* quota / mode privé */
  }
}

export function readLastPortalTenant(): SavedPortalTenant | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as SavedPortalTenant;
    if (!parsed?.slug || !parsed.signInUrl) return null;
    return parsed;
  } catch {
    return null;
  }
}

export function tenantSelectLabel(entry: {
  label: string;
  kindLabel: string;
  postalAddressLabel: string;
}): string {
  const address = entry.postalAddressLabel || "Adresse non renseignée";
  return `${entry.label} — ${entry.kindLabel} — ${address}`;
}

export function clearLastPortalTenant(): void {
  if (typeof window === "undefined") return;
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* ignore */
  }
}

type CatalogSignInEntry = {
  slug: string;
  label: string;
  signInUrl: string;
  primaryHostname?: string | null;
  appUrl?: string;
};

/** Met à jour le tenant mémorisé avec les URLs fraîches du catalogue. */
export function syncSavedPortalTenantFromCatalog(
  catalog: CatalogSignInEntry[],
): SavedPortalTenant | null {
  const saved = readLastPortalTenant();
  if (!saved?.slug) return null;
  const hit = catalog.find((t) => t.slug === saved.slug);
  if (!hit) return null;

  const signInUrl = catalogEntrySignInUrl(hit);
  const refreshed: SavedPortalTenant = {
    slug: hit.slug,
    label: hit.label,
    signInUrl,
    savedAt: saved.savedAt,
  };
  saveLastPortalTenant(refreshed);
  return refreshed;
}

/** Origine intranet établissement (dernier choix mémorisé ou unique tenant du catalogue). */
export async function resolveEstablishmentPortalOrigin(): Promise<string | null> {
  const saved = await resolveSavedPortalTenantSignIn();
  if (saved) {
    try {
      return new URL(saved).origin;
    } catch {
      /* fall through */
    }
  }

  try {
    const res = await fetch("/api/tenants/public", { cache: "no-store" });
    const j = await res.json();
    if (!res.ok) return null;
    const catalog = (j.tenants as CatalogSignInEntry[] | undefined) ?? [];
    if (catalog.length === 1) {
      return new URL(catalogEntrySignInUrl(catalog[0])).origin;
    }
  } catch {
    return null;
  }

  return null;
}

/** Vérifie que le tenant mémorisé existe encore dans le catalogue. */
async function resolveSavedPortalTenantSignIn(): Promise<string | null> {
  const saved = readLastPortalTenant();
  if (!saved) return null;
  try {
    const res = await fetch("/api/tenants/public", { cache: "no-store" });
    const j = await res.json();
    if (!res.ok) return null;
    const catalog = (j.tenants as CatalogSignInEntry[] | undefined) ?? [];
    const refreshed = syncSavedPortalTenantFromCatalog(catalog);
    return refreshed?.signInUrl ?? null;
  } catch {
    return null;
  }
}
