"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import CertificatePendingSignaturesPanel from "@/app/components/certificates/CertificatePendingSignaturesPanel";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { currentCertificateSchoolYear } from "@/app/lib/certificates-types";

type ProgramEntry = {
  id: string;
  title: string;
  schoolYear: string;
  ownerName: string;
  status: string;
  updatedAt: string;
};

export default function CertificatesListPage() {
  const [mine, setMine] = useState<ProgramEntry[]>([]);
  const [shared, setShared] = useState<ProgramEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch("/api/certificates/programs", { cache: "no-store" })
      .then((r) => r.json())
      .then((j) => {
        setMine(j.mine || []);
        setShared(j.shared || []);
      })
      .catch(() => {
        setMine([]);
        setShared([]);
      })
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <ModulePageShell maxWidthClass="max-w-5xl">
        <p className="text-center font-bold text-slate-600">Chargement…</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        title="Parcours & certificats"
        description="Créez des parcours, ajoutez des élèves, des lignes personnalisées et signez en fin d'année."
        actions={
          <>
            <Link
              href="/certificates/my-signature"
              className="rounded-xl bg-slate-100 px-4 py-2.5 text-sm font-black text-slate-700"
            >
              Ma signature
            </Link>
            <Link
              href="/certificates/new"
              className="rounded-xl bg-indigo-600 px-4 py-2.5 text-sm font-black text-white shadow-lg"
            >
              Nouveau parcours
            </Link>
          </>
        }
      />

      <CertificatePendingSignaturesPanel />

      <section>
        <h2 className="text-lg font-black text-slate-900 mb-3">Mes parcours</h2>
        {mine.length === 0 ? (
          <p className="text-sm text-slate-500">Aucun parcours créé.</p>
        ) : (
          <div className="grid gap-3">
            {mine.map((p) => (
              <Link
                key={p.id}
                href={`/certificates/${p.id}`}
                className="block rounded-2xl border border-slate-200 bg-white p-4 hover:border-indigo-300 transition"
              >
                <div className="flex justify-between gap-2">
                  <p className="font-black text-slate-900">{p.title}</p>
                  <span className="text-xs text-slate-500">{p.schoolYear}</span>
                </div>
                <p className="text-xs text-slate-500 mt-1">Créateur : {p.ownerName}</p>
              </Link>
            ))}
          </div>
        )}
      </section>

      {shared.length > 0 && (
        <section>
          <h2 className="text-lg font-black text-slate-900 mb-3">Parcours partagés avec moi</h2>
          <div className="grid gap-3">
            {shared.map((p) => (
              <Link
                key={p.id}
                href={`/certificates/${p.id}`}
                className="block rounded-2xl border border-slate-200 bg-white p-4 hover:border-violet-300 transition"
              >
                <div className="flex justify-between gap-2">
                  <p className="font-black text-slate-900">{p.title}</p>
                  <span className="text-xs text-slate-500">{p.schoolYear}</span>
                </div>
                <p className="text-xs text-slate-500 mt-1">Créateur : {p.ownerName}</p>
              </Link>
            ))}
          </div>
        </section>
      )}

      <p className="text-xs text-slate-400">Année scolaire en cours : {currentCertificateSchoolYear()}</p>
    </ModulePageShell>
  );
}
