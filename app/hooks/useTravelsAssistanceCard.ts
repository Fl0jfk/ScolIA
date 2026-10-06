"use client";

import { useCallback, useEffect, useState } from "react";
import {
  TRAVELS_ASSISTANCE_CARD_CHANGED_EVENT,
  type TravelsAssistanceCardApiStatus,
} from "@/app/lib/travels-assistance-card-shared";

export function notifyTravelsAssistanceCardChanged(): void {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(TRAVELS_ASSISTANCE_CARD_CHANGED_EVENT));
  }
}

/** État serveur frais (pas le bootstrap app context) pour afficher le bouton Assistance. */
export function useTravelsAssistanceCard(enabled = true) {
  const [status, setStatus] = useState<TravelsAssistanceCardApiStatus | null>(null);
  const [loading, setLoading] = useState(enabled);

  const reload = useCallback(async () => {
    if (!enabled) {
      setStatus(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const res = await fetch("/api/travels/assistance-card", { cache: "no-store" });
      if (res.status === 403) {
        setStatus(null);
        return;
      }
      const j = (await res.json()) as TravelsAssistanceCardApiStatus & { error?: string };
      if (!res.ok) {
        setStatus({ configured: false, fileName: null, downloadUrl: null });
        return;
      }
      setStatus({
        configured: Boolean(j.configured),
        fileName: j.fileName ?? null,
        downloadUrl: j.downloadUrl ?? null,
      });
    } catch {
      setStatus(null);
    } finally {
      setLoading(false);
    }
  }, [enabled]);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    if (!enabled) return;
    const onRefresh = () => void reload();
    window.addEventListener(TRAVELS_ASSISTANCE_CARD_CHANGED_EVENT, onRefresh);
    window.addEventListener("focus", onRefresh);
    return () => {
      window.removeEventListener(TRAVELS_ASSISTANCE_CARD_CHANGED_EVENT, onRefresh);
      window.removeEventListener("focus", onRefresh);
    };
  }, [enabled, reload]);

  return { status, loading, reload };
}
