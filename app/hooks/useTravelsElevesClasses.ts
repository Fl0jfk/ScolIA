"use client";

import { useEffect, useState } from "react";

/**
 * Catalogue de classes pour Voyages — même source que l’onglet Élèves
 * (`GET /api/travels/eleves-picker` : élèves scolarisés + filtre année / Siècle).
 */
export function useTravelsElevesClasses(): {
  classOptions: string[];
  loading: boolean;
  error: string | null;
} {
  const [classOptions, setClassOptions] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetch("/api/travels/eleves-picker")
      .then(async (r) => {
        const j = (await r.json()) as { classes?: unknown; error?: string };
        if (!r.ok) throw new Error(j.error || "Chargement des classes impossible");
        if (cancelled) return;
        setClassOptions(Array.isArray(j.classes) ? j.classes.map(String) : []);
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        setClassOptions([]);
        setError(e instanceof Error ? e.message : "Erreur");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return { classOptions, loading, error };
}
