"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import {
  PARTENARIAT_CYCLES,
  PARTENARIAT_CYCLE_LABELS,
  PARTENARIAT_NIVEAUX_BY_CYCLE,
  offreMatchesFilters,
  type PartenariatCycle,
  type PartenariatOffrePublicCard,
} from "@/app/lib/partenariats-types";

type Props = {
  schoolName: string;
  offres: PartenariatOffrePublicCard[];
};

export default function PartenariatsPublicCatalogClient({ schoolName, offres }: Props) {
  const [cycle, setCycle] = useState<PartenariatCycle | null>(null);
  const [niveau, setNiveau] = useState<string | null>(null);
  const [logoFailed, setLogoFailed] = useState(false);

  const niveauxOptions = useMemo(() => {
    if (cycle) return PARTENARIAT_NIVEAUX_BY_CYCLE[cycle];
    const ids = new Set(offres.flatMap((o) => o.niveaux));
    return PARTENARIAT_CYCLES.flatMap((c) =>
      PARTENARIAT_NIVEAUX_BY_CYCLE[c].filter((n) => ids.has(n.id)),
    );
  }, [cycle, offres]);

  const filtered = useMemo(
    () => offres.filter((o) => offreMatchesFilters(o, cycle, niveau)),
    [offres, cycle, niveau],
  );

  const hasAnyNiveau = offres.some((o) => o.niveaux.length > 0);

  return (
    <div className="partenariats-public min-h-screen bg-[radial-gradient(1200px_600px_at_10%_-10%,#dbeafe_0%,transparent_55%),radial-gradient(900px_500px_at_90%_0%,#fef3c7_0%,transparent_50%),linear-gradient(180deg,#f8fafc_0%,#eef2ff_100%)]">
      <div className="mx-auto max-w-5xl px-4 py-10 sm:py-14">
        <header className="mb-10 space-y-3">
          {!logoFailed ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src="/api/site/header-logo"
              alt={schoolName || "Logo de l’établissement"}
              className="h-16 w-auto max-w-[220px] object-contain object-left sm:h-20"
              onError={() => setLogoFailed(true)}
            />
          ) : null}
          <p className="text-xs font-bold uppercase tracking-[0.18em] text-sky-800/80">
            {schoolName || "Établissement"}
          </p>
          <h1 className="font-[family-name:var(--font-partenariats-display)] text-4xl font-semibold tracking-tight text-slate-900 sm:text-5xl">
            Partenariats &amp; offres
          </h1>
          <p className="max-w-2xl text-base text-slate-600 sm:text-lg">
            Découvrez les partenaires et dispositifs proposés, les interlocuteurs à joindre, et
            inscrivez-vous en ligne quand c’est possible.
          </p>
        </header>

        <div className="mb-8 flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => {
              setCycle(null);
              setNiveau(null);
            }}
            className={`rounded-full px-3.5 py-1.5 text-sm font-semibold transition ${
              cycle === null
                ? "bg-slate-900 text-white"
                : "bg-white/80 text-slate-700 ring-1 ring-slate-200 hover:bg-white"
            }`}
          >
            Tous
          </button>
          {PARTENARIAT_CYCLES.map((c) => (
            <button
              key={c}
              type="button"
              onClick={() => {
                setCycle(c);
                setNiveau(null);
              }}
              className={`rounded-full px-3.5 py-1.5 text-sm font-semibold transition ${
                cycle === c
                  ? "bg-slate-900 text-white"
                  : "bg-white/80 text-slate-700 ring-1 ring-slate-200 hover:bg-white"
              }`}
            >
              {PARTENARIAT_CYCLE_LABELS[c]}
            </button>
          ))}
        </div>

        {hasAnyNiveau && niveauxOptions.length > 0 ? (
          <div className="mb-8 flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => setNiveau(null)}
              className={`rounded-lg px-2.5 py-1 text-xs font-bold ${
                niveau === null ? "bg-sky-100 text-sky-900" : "bg-white/70 text-slate-600"
              }`}
            >
              Tous niveaux
            </button>
            {niveauxOptions.map((n) => (
              <button
                key={n.id}
                type="button"
                onClick={() => setNiveau(n.id)}
                className={`rounded-lg px-2.5 py-1 text-xs font-bold ${
                  niveau === n.id ? "bg-sky-100 text-sky-900" : "bg-white/70 text-slate-600"
                }`}
              >
                {n.label}
              </button>
            ))}
          </div>
        ) : null}

        {filtered.length === 0 ? (
          <p className="rounded-2xl bg-white/80 px-5 py-8 text-center text-slate-600 ring-1 ring-slate-200">
            Aucune offre pour ce filtre pour le moment.
          </p>
        ) : (
          <ul className="grid gap-4 sm:grid-cols-2">
            {filtered.map((o) => (
              <li key={o.slug}>
                <Link
                  href={`/partenariats/${o.slug}`}
                  className="group flex h-full flex-col gap-3 rounded-2xl bg-white/90 p-5 shadow-sm ring-1 ring-slate-200/80 transition hover:-translate-y-0.5 hover:shadow-md"
                >
                  <div className="flex items-start gap-3">
                    {o.logoUrl ? (
                      // eslint-disable-next-line @next/next/no-img-element
                      <img
                        src={o.logoUrl}
                        alt=""
                        className="h-14 w-14 shrink-0 rounded-xl object-contain bg-slate-50 ring-1 ring-slate-100"
                      />
                    ) : (
                      <div className="flex h-14 w-14 shrink-0 items-center justify-center rounded-xl bg-slate-100 text-xs font-bold text-slate-400">
                        Logo
                      </div>
                    )}
                    <div className="min-w-0">
                      <p className="font-bold text-slate-900 group-hover:text-sky-800">
                        {o.title}
                      </p>
                      {o.categoryLabel ? (
                        <p className="mt-0.5 text-xs font-semibold uppercase tracking-wide text-sky-700/80">
                          {o.categoryLabel}
                        </p>
                      ) : null}
                    </div>
                  </div>
                  <p className="line-clamp-3 flex-1 text-sm text-slate-600">
                    {o.shortDescription || "En savoir plus"}
                  </p>
                  <div className="flex flex-wrap gap-1.5">
                    <span className="rounded-md bg-slate-100 px-1.5 py-0.5 text-[10px] font-bold uppercase text-slate-600">
                      {o.kind === "inscription" ? "Inscription" : "Info"}
                    </span>
                    {o.cycles.map((c) => (
                      <span
                        key={c}
                        className="rounded-md bg-slate-100 px-1.5 py-0.5 text-[10px] font-bold text-slate-600"
                      >
                        {PARTENARIAT_CYCLE_LABELS[c]}
                      </span>
                    ))}
                  </div>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
