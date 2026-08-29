"use client";

import StageReferentsEditor from "@/app/components/stages/StageReferentsEditor";
import StagePeriodsEditor from "@/app/components/stages/StagePeriodsEditor";

export default function StagesSettingsPanel({
  onSavedMsg,
}: {
  onSavedMsg: (message: string) => void;
}) {
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
          Sélectionnez les classes collège et lycée importées depuis SIECLE, puis configurez leurs
          périodes. L&apos;école primaire n&apos;est pas incluse.
        </p>
        <div className="mt-4">
          <StagePeriodsEditor onSaved={(m) => onSavedMsg(m)} />
        </div>
      </section>

      <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Professeurs principaux / référents par classe</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-2xl">
          Assignez un ou plusieurs professeurs référents pour chaque classe activée ci-dessus. Si
          aucune classe n&apos;est configurée, cette section reste vide.
        </p>
        <div className="mt-4">
          <StageReferentsEditor onSaved={(m) => onSavedMsg(m)} />
        </div>
      </section>
    </div>
  );
}
