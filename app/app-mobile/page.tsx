import Link from "next/link";

export default function AppMobileLandingPage() {
  return (
    <main className="min-h-screen bg-gradient-to-b from-slate-950 via-slate-900 to-slate-800 text-white">
      <div className="mx-auto flex min-h-screen max-w-lg flex-col justify-center px-6 py-16">
        <p className="text-xs font-bold uppercase tracking-[0.2em] text-sky-300/90">ScolIA</p>
        <h1 className="mt-3 text-4xl font-black tracking-tight">Ouvrez l’application</h1>
        <p className="mt-4 text-base leading-relaxed text-slate-300">
          Les comptes famille et élève n’ont pas accès à l’intranet web établissement. Tout passe par
          l’app (iOS / Android) : enfants, absences, bulletins, finances.
        </p>
        <ul className="mt-8 space-y-2 text-sm text-slate-300">
          <li className="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
            Parents → API <code className="text-sky-300">/api/famille</code>
          </li>
          <li className="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
            Élèves → API <code className="text-sky-300">/api/eleve</code>
          </li>
          <li className="rounded-xl border border-white/10 bg-white/5 px-4 py-3">
            Staff lite (appel, EDT) → API <code className="text-sky-300">/api/mobile</code>
          </li>
        </ul>
        <div className="mt-10 flex flex-wrap gap-3">
          <Link
            href="/famille"
            className="rounded-xl bg-sky-500 px-5 py-3 text-sm font-bold text-slate-950 hover:bg-sky-400"
          >
            Continuer (coque famille temporaire)
          </Link>
          <Link
            href="/sign-out"
            className="rounded-xl border border-white/20 px-5 py-3 text-sm font-bold text-white hover:bg-white/10"
          >
            Se déconnecter
          </Link>
        </div>
      </div>
    </main>
  );
}
