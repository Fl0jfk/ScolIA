"use client";

import Link from "next/link";

const EVENTS = [
  {
    id: "portes-ouvertes",
    title: "Portes ouvertes",
    description: "Inscriptions en ligne, créneaux, confirmations et calendrier familles.",
    href: "/toolbox?tab=portes-ouvertes",
    publicHref: "/portes-ouvertes",
    season: "Sept.–janv.",
    accent: "border-violet-200 bg-violet-50/80 text-violet-900",
  },
  {
    id: "rentree",
    title: "Rentrée digitale",
    description: "Hub familles : documents, simulateurs et liens utiles pour la rentrée.",
    href: "/toolbox?tab=rentree",
    publicHref: "/rentree",
    season: "Août–sept.",
    accent: "border-amber-200 bg-amber-50/80 text-amber-900",
  },
  {
    id: "secret-santa",
    title: "Secret Santa",
    description: "Tirage au sort anonyme pour l’équipe ou une classe.",
    href: "/toolbox/secret-santa",
    season: "Décembre",
    accent: "border-rose-200 bg-rose-50/80 text-rose-900",
  },
] as const;

export default function EvenementsHubClient() {
  return (
    <main className="max-w-4xl mx-auto px-4 sm:px-6 py-10 space-y-8">
      <header>
        <p className="text-[10px] font-black uppercase tracking-widest text-amber-700">Établissement</p>
        <h1 className="text-3xl font-black text-slate-900 tracking-tight mt-1">Événements</h1>
        <p className="text-sm text-slate-500 mt-2 max-w-2xl">
          Pilotage des temps forts de l&apos;année — portes ouvertes, rentrée et Secret Santa.
          La configuration détaillée reste accessible depuis la boîte à outils.
        </p>
      </header>

      <div className="grid gap-4 sm:grid-cols-1 md:grid-cols-3">
        {EVENTS.map((ev) => (
          <article
            key={ev.id}
            className={`rounded-2xl border p-5 flex flex-col gap-3 ${ev.accent}`}
          >
            <div className="flex items-start justify-between gap-2">
              <h2 className="text-lg font-black leading-tight">{ev.title}</h2>
              <span className="shrink-0 text-[10px] font-bold uppercase tracking-wide opacity-70">
                {ev.season}
              </span>
            </div>
            <p className="text-sm opacity-90 flex-1">{ev.description}</p>
            <div className="flex flex-wrap gap-2 pt-1">
              <Link
                href={ev.href}
                className="inline-flex rounded-xl bg-slate-900 px-3 py-2 text-xs font-bold text-white hover:bg-slate-800"
              >
                Configurer
              </Link>
              {"publicHref" in ev && ev.publicHref ? (
                <Link
                  href={ev.publicHref}
                  className="inline-flex rounded-xl border border-current/20 bg-white/70 px-3 py-2 text-xs font-bold hover:bg-white"
                >
                  Page publique
                </Link>
              ) : null}
            </div>
          </article>
        ))}
      </div>

      <p className="text-xs text-slate-500">
        Le générateur de QR code reste dans la{" "}
        <Link href="/toolbox" className="font-semibold text-slate-700 underline-offset-2 hover:underline">
          boîte à outils
        </Link>
        .
      </p>
    </main>
  );
}
