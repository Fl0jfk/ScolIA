"use client";

import { createContext, useContext, PropsWithChildren, useEffect, useState } from "react";
import {
  getDashboardCategories,
  type DashboardCategory,
  type DashboardTileVariant,
  type ExternalQuickLink,
} from "@/app/lib/intranet-modules";
import {
  readDashboardLinksCache,
  writeDashboardLinksCache,
} from "@/app/lib/dashboard-links-cache";

type Categories = DashboardCategory;

type Data = {
  categories: DashboardCategory[];
  externalQuickLinks: ExternalQuickLink[];
  travels: [];
  documents: [];
  error: null;
};

const DataContext = createContext<Data | undefined>(undefined);

export const DataProvider = ({ children }: PropsWithChildren<object>) => {
  const [data, setData] = useState<Data>({
    categories: getDashboardCategories(),
    externalQuickLinks: [],
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

  return <DataContext.Provider value={data}>{children}</DataContext.Provider>;
};

export const useData = () => {
  const context = useContext(DataContext);
  if (!context) throw new Error("useData must be used within a DataProvider");
  return context;
};
