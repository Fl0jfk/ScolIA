"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type ClientAppUser = {
  id: string;
  businessUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  name?: string;
  imageUrl?: string;
  roles: string[];
  orgAdmin: boolean;
  platformAdmin: boolean;
  isSignedIn: boolean;
  authSource: "better-auth" | "none";
  externalUserId?: string;
};

/** Forme compatible avec l’ancien usage `useSessionUser()` (publicMetadata.role). */
export type SessionUserView = {
  id: string;
  firstName: string | null;
  lastName: string | null;
  fullName: string | null;
  username: string | null;
  imageUrl: string | null;
  primaryEmailAddress: { emailAddress: string } | null;
  publicMetadata: { role: string[]; org_admin?: boolean; platform_admin?: boolean };
};

type AppUserState = {
  isLoaded: boolean;
  user: ClientAppUser | null;
};

type AppUserContextValue = AppUserState & { refresh: () => Promise<void> };

const AppUserContext = createContext<AppUserContextValue | null>(null);

function mapApiUser(u: {
  id: string;
  businessUserId?: string;
  email: string;
  name?: string;
  firstName?: string;
  lastName?: string;
  imageUrl?: string;
  externalUserId?: string;
  roles?: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
  authSource?: "better-auth";
}): ClientAppUser {
  const roles = Array.isArray(u.roles) ? u.roles : [];
  return {
    id: u.id,
    businessUserId: u.businessUserId?.trim() || u.externalUserId?.trim() || u.id,
    email: u.email,
    firstName: u.firstName,
    lastName: u.lastName,
    name: u.name,
    imageUrl: u.imageUrl,
    roles,
    orgAdmin: Boolean(u.orgAdmin || u.platformAdmin || roles.includes("admin")),
    platformAdmin: Boolean(u.platformAdmin),
    isSignedIn: true,
    authSource: u.authSource ?? "better-auth",
    externalUserId: u.externalUserId,
  };
}

/** Une seule source de vérité session client — évite les courses RequireOrgAdmin / hooks. */
export function AppUserProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AppUserState>({ isLoaded: false, user: null });

  const refresh = useCallback(async () => {
    try {
      const res = await fetch("/api/auth/me", { credentials: "include", cache: "no-store" });
      if (!res.ok) {
        setState({ isLoaded: true, user: null });
        return;
      }
      const data = (await res.json()) as { user?: Parameters<typeof mapApiUser>[0] | null };
      if (!data?.user) {
        setState({ isLoaded: true, user: null });
        return;
      }
      setState({ isLoaded: true, user: mapApiUser(data.user) });
    } catch {
      setState({ isLoaded: true, user: null });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const value = useMemo(() => ({ ...state, refresh }), [state, refresh]);

  return <AppUserContext.Provider value={value}>{children}</AppUserContext.Provider>;
}

/**
 * Session client unifiée via `/api/auth/me` (Better-Auth).
 * Doit être utilisé sous `AppUserProvider`.
 */
export function useAppUser(): AppUserContextValue {
  const ctx = useContext(AppUserContext);
  if (!ctx) {
    throw new Error("useAppUser doit être utilisé dans AppUserProvider");
  }
  return ctx;
}

/** Remplace `useSessionUser()` dans les écrans intranet (sans provider legacyProvider). */
export function useSessionUser(): {
  isLoaded: boolean;
  isSignedIn: boolean;
  user: SessionUserView | null;
} {
  const { isLoaded, user } = useAppUser();

  const view = useMemo((): SessionUserView | null => {
    if (!user) return null;
    const fullName =
      user.name?.trim() ||
      `${user.firstName ?? ""} ${user.lastName ?? ""}`.trim() ||
      user.email;
    return {
      id: user.businessUserId || user.id,
      firstName: user.firstName ?? null,
      lastName: user.lastName ?? null,
      fullName: fullName || null,
      username: user.email,
      imageUrl: user.imageUrl ?? null,
      primaryEmailAddress: user.email ? { emailAddress: user.email } : null,
      publicMetadata: {
        role: user.roles,
        org_admin: user.orgAdmin,
        platform_admin: user.platformAdmin,
      },
    };
  }, [user]);

  return {
    isLoaded,
    isSignedIn: Boolean(user?.isSignedIn),
    user: view,
  };
}
