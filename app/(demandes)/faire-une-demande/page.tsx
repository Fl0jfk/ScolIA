"use client";

import Link from "@/app/components/ui/AppLink";
import FaireUneDemandeForm from "@/app/components/requests/FaireUneDemandeForm";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";

export default function FaireUneDemandePage() {
  return (
    <div className="min-h-screen">
      <RentreePublicHeader />
      <main className="max-w-2xl mx-auto px-6 py-10">
        <div className="mb-8">
          <p className="text-xs font-bold uppercase tracking-widest text-blue-600">Établissement</p>
          <h1 className="text-4xl font-black text-slate-900 mt-2">Faire une demande</h1>
          <p className="text-slate-600 mt-3 leading-relaxed">
            Décrivez simplement votre besoin. Pas de catégorie à choisir : le système oriente la demande vers le bon service.
          </p>
        </div>

        <FaireUneDemandeForm variant="page" mesDemandesHref="/requests#mes-demandes" />

        <p className="text-center text-xs text-slate-400 mt-8">
          Personnel de l&apos;établissement ?{" "}
          <Link href="/sign-in" className="text-blue-600 font-bold hover:underline">
            Se connecter
          </Link>{" "}
          pour accéder à la page Demandes (dépôt + suivi).
        </p>
      </main>
    </div>
  );
}
