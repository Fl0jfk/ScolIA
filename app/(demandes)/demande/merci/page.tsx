"use client";

import Link from "@/app/components/ui/AppLink";
import { useSearchParams } from "next/navigation";
import { Suspense } from "react";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";

function MerciContent() {
  const params = useSearchParams();
  const ok = params.get("ok") === "1";
  const id = params.get("id") || "";
  const erreur = params.get("erreur") || "";

  const messages: Record<string, { title: string; body: string }> = {
    lien_invalide: { title: "Lien invalide", body: "Ce lien de confirmation n'est plus valable. Vous pouvez refaire une demande." },
    lien_expire: { title: "Lien expiré", body: "Le délai de confirmation (72 h) est dépassé. Merci de soumettre à nouveau votre demande." },
    serveur: { title: "Erreur technique", body: "Une erreur est survenue. Réessayez dans quelques instants ou contactez l'établissement." },
  };

  if (ok) {
    return (
      <div className="rounded-3xl border border-emerald-200 bg-emerald-50 p-8 text-center space-y-4">
        <p className="text-4xl">✓</p>
        <h1 className="text-2xl font-black text-emerald-900">Demande confirmée</h1>
        <p className="text-emerald-800 text-sm leading-relaxed">
          Votre demande a bien été transmise à l&apos;établissement. Vous recevrez un accusé de réception par e-mail.
        </p>
        {id ? <p className="text-xs font-mono text-emerald-700">Réf. {id}</p> : null}
        <Link href="/faire-une-demande" className="inline-block mt-4 px-6 py-3 rounded-full bg-emerald-600 text-white font-bold text-sm hover:bg-emerald-700 transition">
          Retour
        </Link>
      </div>
    );
  }

  const err = messages[erreur];
  if (err) {
    return (
      <div className="rounded-3xl border border-red-200 bg-red-50 p-8 text-center space-y-4">
        <h1 className="text-2xl font-black text-red-900">{err.title}</h1>
        <p className="text-red-800 text-sm leading-relaxed">{err.body}</p>
        <Link href="/faire-une-demande" className="inline-block mt-4 px-6 py-3 rounded-full bg-slate-900 text-white font-bold text-sm hover:bg-black transition">
          Nouvelle demande
        </Link>
      </div>
    );
  }

  return (
    <div className="rounded-3xl border border-slate-200 bg-white p-8 text-center">
      <Link href="/faire-une-demande" className="text-blue-600 font-bold text-sm">Faire une demande</Link>
    </div>
  );
}

export default function DemandeMerciPage() {
  return (
    <div>
      <RentreePublicHeader />
      <main className="max-w-lg mx-auto px-6 py-16">
        <Suspense fallback={<p className="text-center text-slate-500 animate-pulse">Chargement…</p>}>
          <MerciContent />
        </Suspense>
      </main>
    </div>
  );
}
