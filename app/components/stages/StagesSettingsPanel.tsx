"use client";

import StageReferentsEditor from "@/app/components/stages/StageReferentsEditor";
import StagePeriodsEditor from "@/app/components/stages/StagePeriodsEditor";
import StageWatchersEditor from "@/app/components/stages/StageWatchersEditor";
import StageConstraintsEditor from "@/app/components/stages/StageConstraintsEditor";

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
          Les élèves s&apos;identifient (nom, prénom et date de naissance), puis remplissent le formulaire
          en ligne : entreprise, horaires, dates, contacts — sans dépôt de PDF.
        </p>
        <a
          href="/stages/preconvention"
          target="_blank"
          rel="noopener noreferrer"
          className="mt-2 block rounded-lg border border-emerald-100 bg-white px-3 py-2 text-sm font-mono break-all text-[#2F6B4A] underline hover:bg-emerald-50"
        >
          {typeof window !== "undefined" ? `${window.location.origin}/stages/preconvention` : "/stages/preconvention"}
        </a>
      </section>

      <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Horaires, jours et périodes bloquées</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-2xl">
          Limitez les plages horaires (ex. 6h–20h au collège), les jours (pas de week-end au
          collège, samedi possible au lycée), les plafonds d&apos;heures selon l&apos;âge, et
          interdisez certaines dates (établissement fermé).
        </p>
        <div className="mt-4">
          <StageConstraintsEditor onSaved={(m) => onSavedMsg(m)} />
        </div>
      </section>

      <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Classes concernées par les stages</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-2xl">
          Sélectionnez les classes collège et lycée importées depuis SIECLE, puis configurez leurs
          périodes officielles (rappel informatif). Une classe sans période reste éligible au dépôt
          volontaire. L&apos;école primaire n&apos;est pas incluse.
        </p>
        <div className="mt-4">
          <StagePeriodsEditor onSaved={(m) => onSavedMsg(m)} />
        </div>
      </section>

      <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Professeurs principaux / référents par classe</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-2xl">
          Assignez un professeur principal et, si besoin, un ou plusieurs référents stage. Une classe
          hors liste (ex. terminale) s&apos;ajoute toute seule dès qu&apos;un élève y dépose une
          préconvention — vous pourrez ensuite y rattacher les professeurs.
        </p>
        <div className="mt-4">
          <StageReferentsEditor onSaved={(m) => onSavedMsg(m)} />
        </div>
      </section>

      <section className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-bold text-[#1F3D2B]">CPE &amp; restauration (visibilité)</h2>
        <p className="mt-2 text-sm text-stone-600 max-w-2xl">
          Affectez les CPE par classe (ou élève hors parcours) pour le suivi des stages. Affectez
          aussi la restauration pour connaître les jours où les élèves ne mangeront pas à
          l&apos;établissement.
        </p>
        <div className="mt-4">
          <StageWatchersEditor onSaved={(m) => onSavedMsg(m)} />
        </div>
      </section>
    </div>
  );
}
