import Link from "next/link";
import { redirect } from "next/navigation";
import {
  DEMO_PARENT,
  DEMO_STAFF,
  isDemoClickAllowed,
} from "@/app/lib/demo-click";

export const dynamic = "force-dynamic";

type Props = {
  searchParams: Promise<{ error?: string }>;
};

export default async function DemoPage({ searchParams }: Props) {
  if (!isDemoClickAllowed()) {
    redirect("/");
  }

  const { error } = await searchParams;

  return (
    <main className="min-h-[100dvh] bg-gradient-to-b from-teal-950 via-slate-900 to-slate-950 text-slate-50">
      <div className="mx-auto flex min-h-[100dvh] max-w-lg flex-col justify-center gap-8 px-5 py-12">
        <header className="space-y-3 text-center">
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-teal-300/90">
            ScolIA · ENT du matin · 25 %
          </p>
          <h1 className="text-3xl font-semibold tracking-tight text-white sm:text-4xl">
            Démo labo
          </h1>
          <p className="text-sm leading-relaxed text-slate-300">
            Données fictives (Leo JUSTIF, classe 4B). Hors production — un clic
            pour entrer, puis balade-toi.
          </p>
        </header>

        {error ? (
          <p
            role="alert"
            className="rounded-lg border border-rose-400/40 bg-rose-950/50 px-4 py-3 text-sm text-rose-100"
          >
            {error}
          </p>
        ) : null}

        <div className="flex flex-col gap-3">
          <a
            href="/api/demo/enter?as=parent&dev_tenant=default"
            className="rounded-xl bg-teal-400 px-5 py-4 text-center text-base font-semibold text-teal-950 shadow-lg shadow-teal-950/40 transition hover:bg-teal-300 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-teal-200"
          >
            Entrer comme parent → Portail quotidien
          </a>
          <a
            href="/api/demo/enter?as=staff&dev_tenant=default"
            className="rounded-xl border border-slate-500/60 bg-slate-800/80 px-5 py-4 text-center text-base font-semibold text-slate-50 transition hover:border-teal-400/50 hover:bg-slate-700 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-teal-200"
          >
            Entrer comme staff → Intranet
          </a>
        </div>

        <section className="space-y-3 rounded-xl border border-slate-600/50 bg-slate-900/60 px-4 py-4 text-sm text-slate-300">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
            À essayer (matin)
          </h2>
          <ul className="list-inside list-disc space-y-1.5">
            <li>Parent : EDT, notes, cahier, messages, carnet, sanctions</li>
            <li>Staff : appel VS, saisie notes, cahier de textes, carnet</li>
          </ul>
          <p className="border-t border-slate-700/80 pt-3 text-xs text-slate-500">
            Parent : {DEMO_PARENT.email} · Staff : {DEMO_STAFF.email} (TOTP
            auto)
          </p>
        </section>

        <p className="text-center text-xs text-slate-500">
          Déjà connecté ?{" "}
          <Link href="/quotidien" className="text-teal-300 underline-offset-2 hover:underline">
            /quotidien
          </Link>
          {" · "}
          <Link href="/dashboard" className="text-teal-300 underline-offset-2 hover:underline">
            /dashboard
          </Link>
        </p>
      </div>
    </main>
  );
}
