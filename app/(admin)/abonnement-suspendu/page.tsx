"use client";

import Link from "next/link";
import { MARKETING } from "@/app/lib/marketing-site";

export default function AbonnementSuspenduPage() {
  return (
    <main className="mx-auto max-w-lg px-6 py-16 text-center">
      <div className="rounded-2xl border border-amber-200 bg-amber-50/90 p-8 shadow-sm">
        <p className="text-xs font-bold uppercase tracking-widest text-amber-800">Abonnement</p>
        <h1 className="mt-2 text-2xl font-black text-[#14231A]">Accès suspendu</h1>
        <p className="mt-4 text-sm text-stone-700 leading-relaxed">
          L&apos;accès à votre espace ScolIA est temporairement suspendu pour cause d&apos;impayé ou de
          décision administrative.
        </p>
        <p className="mt-3 text-sm text-stone-600 leading-relaxed">
          <strong>Vos données sont conservées</strong> (configuration, fichiers sur nos serveurs). Les
          documents stockés sur OneDrive restent sur votre tenant Microsoft — pensez à maintenir une
          copie locale sur vos postes.
        </p>
        <p className="mt-6 text-sm text-stone-700">
          Pour régulariser :{" "}
          <a className="font-semibold text-[#2F6B4A] hover:underline" href={`mailto:${MARKETING.contactEmail}`}>
            {MARKETING.contactEmail}
          </a>
        </p>
        <Link
          href="/assistance"
          className="mt-6 inline-block rounded-xl bg-[#2F6B4A] px-5 py-2.5 text-sm font-semibold text-white hover:bg-[#255A3D]"
        >
          Contacter l&apos;assistance
        </Link>
      </div>
    </main>
  );
}
