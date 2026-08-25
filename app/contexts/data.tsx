"use client";

import { createContext, useContext, PropsWithChildren, useEffect, useState } from "react";
import {
  getDashboardCategories,
  type DashboardCategory,
  type ExternalQuickLink,
} from "@/app/lib/intranet-modules";
import {
  readDashboardLinksCache,
  writeDashboardLinksCache,
} from "@/app/lib/dashboard-links-cache";

type Data = {
  categories: DashboardCategory[];
  externalQuickLinks: ExternalQuickLink[];
  /** Modules effectivement accessibles (matrice tenant + rôles). Null = pas encore chargé. */
  accessibleModuleIds: Set<string> | null;
  travels: [];
  documents: [];
  error: null;
};

const DataContext = createContext<Data | undefined>(undefined);

export const DataProvider = ({ children }: PropsWithChildren<object>) => {
  const [data, setData] = useState<Data>({
    categories: getDashboardCategories(),
    externalQuickLinks: [],
    accessibleModuleIds: null,
    travels: [],
    documents: [],
    error: null,
  });

  useEffect(() => {
    const cachedLinks = readDashboardLinksCache();
    if (cachedLinks) {
      setData((prev) => ({ ...prev, externalQuickLinks: cachedLinks }));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/dashboard/links", { cache: "no-store" });
        const j = await res.json();
        if (!cancelled && res.ok && Array.isArray(j.links)) {
          setData((prev) => ({ ...prev, externalQuickLinks: j.links }));
          writeDashboardLinksCache(j.links);
        }
      } catch {
        /* liens optionnels */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch("/api/me/module-access", { cache: "no-store" });
        const j = (await res.json()) as { moduleIds?: string[] };
        if (!cancelled && res.ok && Array.isArray(j.moduleIds)) {
          setData((prev) => ({
            ...prev,
            accessibleModuleIds: new Set(j.moduleIds),
          }));
        } else if (!cancelled) {
          setData((prev) => ({ ...prev, accessibleModuleIds: new Set() }));
        }
      } catch {
        if (!cancelled) {
          setData((prev) => ({ ...prev, accessibleModuleIds: new Set() }));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return <DataContext.Provider value={data}>{children}</DataContext.Provider>;
};

export const useData = () => {
  const context = useContext(DataContext);
  if (!context) throw new Error("useData must be used within a DataProvider");
  return context;
};
