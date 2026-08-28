"use client";

import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Enfant = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
  foyers: Array<{ id: string; label: string }>;
};

type FoyerSummary = {
  id: string;
  label: string;
  enfantIds: string[];
};

type PortailContext = {
  enfants: Enfant[];
  foyers: FoyerSummary[];
  anneeCouranteLabel: string | null;
  email: string;
};

type Props = {
  title: string;
  description?: string;
  children: (ctx: {
    enfants: Enfant[];
    selectedEnfantId: string | null;
    selectedEnfant: Enfant | null;
    foyers: FoyerSummary[];
    anneeCouranteLabel: string | null;
  }) => ReactNode;
};

export default function FamillePortailChrome({ title, description, children }: Props) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const [data, setData] = useState<PortailContext | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectedEnfantId = searchParams.get("enfant")?.trim() || null;

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/famille/context", { cache: "no-store" });
      const j = (await res.json()) as PortailContext & { error?: string };
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setData(j);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const enfants = data?.enfants ?? [];
  const selectedEnfant = useMemo(() => {
    if (!enfants.length) return null;
    if (selectedEnfantId) {
      return enfants.find((e) => e.id === selectedEnfantId) ?? enfants[0]!;
    }
    return enfants[0]!;
  }, [enfants, selectedEnfantId]);

  const activeFoyers = useMemo(() => {
    if (!selectedEnfant) return data?.foyers ?? [];
    const ids = new Set(selectedEnfant.foyers.map((f) => f.id));
    return (data?.foyers ?? []).filter((f) => ids.has(f.id));
  }, [data?.foyers, selectedEnfant]);

  function setSelectedEnfant(id: string) {
    const p = new URLSearchParams(searchParams.toString());
    p.set("enfant", id);
    router.replace(`${pathname}?${p.toString()}`, { scroll: false });
  }

  return (
    <div className="min-h-screen bg-gradient-to-b from-indigo-50 to-white">
      <header className="border-b border-indigo-100 bg-white/90 backdrop-blur px-4 py-4">
        <div className="max-w-2xl mx-auto">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">
              Espace famille
              {data?.anneeCouranteLabel ? (
                <span className="ml-2 rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-bold text-slate-600 normal-case">
                  {data.anneeCouranteLabel}
                </span>
              ) : null}
            </p>
            {enfants.length > 1 ? (
              <label className="text-xs font-semibold text-slate-600">
                Enfant
                <select
                  value={selectedEnfant?.id ?? ""}
                  onChange={(e) => setSelectedEnfant(e.target.value)}
                  className="ml-2 rounded-lg border border-slate-200 bg-white px-2 py-1 text-xs font-bold text-slate-800"
                >
                  {enfants.map((e) => (
                    <option key={e.id} value={e.id}>
                      {e.prenom} {e.nom}
                      {e.classe ? ` (${e.classe})` : ""}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
          </div>
          <h1 className="text-2xl font-black text-slate-900 mt-0.5">{title}</h1>
          {description ? <p className="text-sm text-slate-600 mt-1">{description}</p> : null}
          {activeFoyers.length > 0 ? (
            <p className="mt-2 text-xs text-indigo-900/80 rounded-xl bg-indigo-50 border border-indigo-100 px-3 py-2">
              Vous agissez pour{" "}
              <strong>{activeFoyers.map((f) => f.label).join(", ")}</strong>
              {data?.email ? (
                <span className="text-indigo-700/70"> — connecté·e en tant que {data.email}</span>
              ) : null}
              . Chaque responsable du foyer peut utiliser son propre compte.
            </p>
          ) : null}
          <FamilleNav enfantId={selectedEnfant?.id ?? null} />
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-6 space-y-4">
        {error ? (
          <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-red-800 text-sm">
            {error}
            <p className="mt-2">
              <a href="/auth/sign-in" className="font-bold underline">
                Se connecter
              </a>
            </p>
          </div>
        ) : !data ? (
          <p className="text-sm text-slate-500">Chargement…</p>
        ) : (
          children({
            enfants,
            selectedEnfantId: selectedEnfant?.id ?? null,
            selectedEnfant,
            foyers: activeFoyers,
            anneeCouranteLabel: data.anneeCouranteLabel,
          })
        )}
      </main>
    </div>
  );
}
