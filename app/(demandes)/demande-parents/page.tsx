"use client";

import { useEffect, useState } from "react";
import ParentDemandeForm from "@/app/components/requests/ParentDemandeForm";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";

export default function DemandeParentsPage() {
  const [loading, setLoading] = useState(true);
  const [enabled, setEnabled] = useState(false);
  const [schoolName, setSchoolName] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [portalRes, siteRes] = await Promise.all([
          fetch("/api/requests/parent-portal", { cache: "no-store" }),
          fetch("/api/site/public", { cache: "no-store" }),
        ]);
        const portal = await portalRes.json();
        const site = siteRes.ok ? await siteRes.json() : null;
        if (cancelled) return;
        setEnabled(portal?.enabled === true);
        setSchoolName(typeof site?.name === "string" ? site.name : null);
      } catch {
        if (!cancelled) setEnabled(false);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="min-h-screen bg-stone-50">
      <RentreePublicHeader />
      <main className="mx-auto max-w-xl px-6 py-10">
        {loading ? (
          <p className="text-center text-sm text-slate-500">Chargement…</p>
        ) : !enabled ? (
          <div className="rounded-2xl border border-slate-200 bg-white p-8 text-center shadow-sm">
            <h1 className="text-2xl font-black text-slate-900">Page non disponible</h1>
            <p className="mt-3 text-sm leading-relaxed text-slate-600">
              L&apos;établissement n&apos;a pas ouvert les demandes en ligne pour les familles pour
              le moment. Contactez l&apos;accueil ou le secrétariat.
            </p>
          </div>
        ) : (
          <>
            <div className="mb-8 text-center">
              <p className="text-xs font-bold uppercase tracking-widest text-amber-800">
                {schoolName || "Établissement"}
              </p>
              <h1 className="mt-2 text-3xl font-black tracking-tight text-slate-900">
                Demande aux familles
              </h1>
              <p className="mt-3 text-sm leading-relaxed text-slate-600">
                Décrivez votre besoin. Après envoi, un e-mail de confirmation validera votre
                demande.
              </p>
            </div>
            <ParentDemandeForm />
          </>
        )}
      </main>
    </div>
  );
}
