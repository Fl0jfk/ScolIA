import {
  isBrowserLocalDev,
  localDevDashboardUrl,
  localDevSignInUrl,
} from "@/app/lib/local-dev";

const STORAGE_KEY = "scola.portal.lastTenant";

type CatalogEntryRef = {
  slug?: string;
  signInUrl: string;
  primaryHostname?: string | null;
  appUrl?: string;
};

function catalogEntryOrigin(entry: CatalogEntryRef): string | null {
  if (isBrowserLocalDev()) {
    return typeof window !== "undefined" ? window.location.origin : null;
  }

  const host = entry.primaryHostname?.trim().replace(/^https?:\/\//, "").replace(/\/$/, "");
  if (host && host !== "localhost" && host !== "127.0.0.1") {
    return `https://${host}`;
  }

  const appUrl = entry.appUrl?.trim().replace(/\/$/, "");
  if (appUrl) {
    try {
      const origin = appUrl.startsWith("http") ? appUrl : `https://${appUrl}`;
      return new URL(origin).origin;
    } catch {
      /* fall through */
    }
  }

  try {
    if (entry.signInUrl.startsWith("http")) return new URL(entry.signInUrl).origin;
  } catch {
    /* ignore */
  }
  return null;
}

/** URL sign-in fiable : en local on reste sur localhost avec le bon tenant. */
export function catalogEntrySignInUrl(entry: CatalogEntryRef): string {
  if (isBrowserLocalDev()) {
    if (entry.signInUrl.startsWith("/")) {
      const url = new URL(entry.signInUrl, window.location.origin);
      return url.toString();
    }
    return localDevSignInUrl(window.location.origin, entry.slug);
  }

  const origin = catalogEntryOrigin(entry);
  if (origin) return `${origin}/auth/sign-in`;
  return entry.signInUrl.includes("/auth/sign-in")
    ? entry.signInUrl
    : entry.signInUrl.replace(/\/sign-in(?:\?|$)/, "/auth/sign-in$1") || "/auth/sign-in";
}

/** Dashboard établissement (session déjà active → pas de formulaire mot de passe). */
export function catalogEntryDashboardUrl(entry: CatalogEntryRef): string {
  if (isBrowserLocalDev()) {
    return localDevDashboardUrl(window.location.origin);
  }
  const origin = catalogEntryOrigin(entry);
  if (origin) return `${origin}/dashboard`;
  return "/dashboard";
}

/** true si une session Better-Auth est active sur ce domaine (cookie partagé *.scolia.fr). */
export async function fetchPortalSessionActive(): Promise<boolean> {
  try {
    const res = await fetch("/api/auth/me", { credentials: "include", cache: "no-store" });
    if (!res.ok) return false;
    const data = (await res.json()) as { user?: unknown };
    return Boolean(data?.user);
  } catch {
    return false;
  }
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
