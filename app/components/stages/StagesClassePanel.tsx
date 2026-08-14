"use client";

import StageClassRosterPanel from "@/app/components/stages/StageClassRosterPanel";

export default function StagesClassePanel({
  defaultSchoolYear,
  onOpenConvention,
  canFileOneDrive,
  oneDriveConnected,
  onFileOneDrive,
  filingConventionId,
}: {
  defaultSchoolYear: string;
  onOpenConvention: (id: string) => void;
  canFileOneDrive: boolean;
  oneDriveConnected: boolean;
  onFileOneDrive: (id: string) => void;
  filingConventionId: string | null;
}) {
  return (
    <section data-tour="stages-classe" className="rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
      <h2 className="text-lg font-bold text-[#1F3D2B]">Suivi des stages par classe</h2>
      <p className="mt-2 text-sm text-stone-600 max-w-3xl">
        Liste de la classe : élèves avec stage validé, en cours de traitement, sans stage, ou avec
        plusieurs conventions. Cliquez sur un dossier pour ouvrir le détail.
      </p>
      <div className="mt-6">
        <StageClassRosterPanel
          defaultSchoolYear={defaultSchoolYear}
          onOpenConvention={onOpenConvention}
          canFileOneDrive={canFileOneDrive}
          oneDriveConnected={oneDriveConnected}
          onFileOneDrive={onFileOneDrive}
          filingConventionId={filingConventionId}
        />
      </div>
    </section>
  );
}
