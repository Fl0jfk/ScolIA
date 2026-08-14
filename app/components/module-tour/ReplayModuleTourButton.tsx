"use client";

import { useModuleTour } from "@/app/components/module-tour/ModuleTourProvider";
import { getModuleTour } from "@/app/lib/module-tours";

export default function ReplayModuleTourButton({ moduleId }: { moduleId: string }) {
  const { requestReplay } = useModuleTour();
  const tour = getModuleTour(moduleId);
  if (!tour) return null;

  return (
    <button
      type="button"
      onClick={() => requestReplay(moduleId)}
      className="text-xs font-semibold text-slate-500 hover:text-emerald-700 underline"
    >
      Revoir le tutoriel de ce module
    </button>
  );
}
