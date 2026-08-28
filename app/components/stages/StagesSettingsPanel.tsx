"use client";

import StageReferentsEditor from "@/app/components/stages/StageReferentsEditor";
import StagePeriodsEditor from "@/app/components/stages/StagePeriodsEditor";

function currentSchoolYearLabel() {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 8) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}

export default function StagesSettingsPanel({
  schoolYear,
  onSavedMsg,
}: {
  schoolYear?: string;
  onSavedMsg: (message: string) => void;
}) {
  const year = schoolYear || currentSchoolYearLabel();

  return (
    <div data-tour="stages-settings" className="space-y-8">
      <section className="rounded-2xl border border-emerald-200 bg-emerald-50/60 p-5">
        <h2 className="text-sm font-bold text-emerald-900">Lien public — formulaire élève</h2>
        <p className="mt-2 text-sm text-emerald-800">
          Les élèves s&apos;identifient (INE + date de naissance), puis remplissent le formulaire
          en ligne : entreprise, horaires, dates, contacts — sans dépôt de PDF.
        </p>
        <p className="mt-2 rounded-lg bg-white border border-emerald-100 px-3 py-2 text-sm font-mono break-all text-[#1F3D2B]">
          {typeof window !== "undefined" ? window.location.origin : ""}/stages/preconvention
        </p>
      </section>

      <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Classes concernées par les stages</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-2xl">
          Ajoutez uniquement les classes qui feront des stages cette année (ex. toutes les 4ᵉ, 3ᵉ,
          2nde, 1ʳᵉ). Les autres classes n&apos;apparaissent ni ici ni sur le formulaire public.
        </p>
        <div className="mt-4">
          <StagePeriodsEditor initialYear={year} onSaved={(m) => onSavedMsg(m)} />
        </div>
      </section>

      <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Professeurs principaux / référents par classe</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-2xl">
          Assignez le professeur principal (ou référent stage) de chaque classe activée. Il verra
          l&apos;onglet <strong>Suivi classe</strong> avec tous les élèves et l&apos;état de leurs
          conventions, et recevra les demandes de signature par e-mail.
        </p>
        <div className="mt-4">
          <StageReferentsEditor initialYear={year} onSaved={(m) => onSavedMsg(m)} />
        </div>
      </section>
    </div>
  );
}
