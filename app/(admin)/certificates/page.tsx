"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import CertificatePendingSignaturesPanel from "@/app/components/certificates/CertificatePendingSignaturesPanel";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";
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
      <ModulePageShell maxWidthClass="max-w-[1280px]">
        <p className={`text-center text-sm ${dash.textMid}`}>Chargement…</p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1280px]">
      <ModulePageHeader
        title="Parcours & certificats"
        description="Créez des parcours, ajoutez des élèves, des lignes personnalisées et signez en fin d'année."
        actions={
          <>
            <Link href="/certificates/my-signature">
              <ModuleButton variant="secondary">Ma signature</ModuleButton>
            </Link>
            <Link href="/certificates/new">
              <ModuleButton>Nouveau parcours</ModuleButton>
            </Link>
          </>
        }
      />

      <CertificatePendingSignaturesPanel />

      <section className="space-y-3">
        <h2 className={`text-lg font-semibold ${dash.ink}`}>Mes parcours</h2>
        {mine.length === 0 ? (
          <p className={`text-sm ${dash.textMid}`}>Aucun parcours créé.</p>
        ) : (
          <div className="grid gap-3">
            {mine.map((p) => (
              <Link key={p.id} href={`/certificates/${p.id}`}>
                <ModuleCard
                  bodyClassName={`p-4 transition hover:-translate-y-0.5 ${dash.hoverBorder}`}
                  className="block"
                >
                  <div className="flex justify-between gap-2">
                    <p className={`font-semibold ${dash.ink}`}>{p.title}</p>
                    <span className={`text-xs ${dash.textMid}`}>{p.schoolYear}</span>
                  </div>
                  <p className={`mt-1 text-xs ${dash.textMid}`}>Créateur : {p.ownerName}</p>
                </ModuleCard>
              </Link>
            ))}
          </div>
        )}
      </section>

      {shared.length > 0 && (
        <section className="mt-6 space-y-3">
          <h2 className={`text-lg font-semibold ${dash.ink}`}>Parcours partagés avec moi</h2>
          <div className="grid gap-3">
            {shared.map((p) => (
              <Link key={p.id} href={`/certificates/${p.id}`}>
                <ModuleCard bodyClassName="p-4 transition hover:-translate-y-0.5" className="block">
                  <div className="flex justify-between gap-2">
                    <p className={`font-semibold ${dash.ink}`}>{p.title}</p>
                    <span className={`text-xs ${dash.textMid}`}>{p.schoolYear}</span>
                  </div>
                  <p className={`mt-1 text-xs ${dash.textMid}`}>Créateur : {p.ownerName}</p>
                </ModuleCard>
              </Link>
            ))}
          </div>
        </section>
      )}

      <p className={`mt-6 text-xs ${dash.textMid}`}>Année scolaire en cours : {currentCertificateSchoolYear()}</p>
    </ModulePageShell>
  );
}
