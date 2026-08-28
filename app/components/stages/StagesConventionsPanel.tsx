"use client";

import { useMemo } from "react";
import type { StageConvention } from "@/app/lib/stage-types";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import type { StagesHubPermissions } from "@/app/components/stages/stages-hub-types";

export default function StagesConventionsPanel({
  conventions,
  permissions,
  oneDriveEnabled,
  oneDriveConnected,
  filingConventionId,
  busy,
  onLoadDetail,
  onFileOneDrive,
}: {
  conventions: StageConvention[];
  permissions: StagesHubPermissions | undefined;
  oneDriveEnabled: boolean;
  oneDriveConnected: boolean;
  filingConventionId: string | null;
  busy: boolean;
  onLoadDetail: (id: string) => void;
  onFileOneDrive: (id: string) => void;
}) {
  const dossiers = useMemo(() => {
    const map = new Map<string, StageConvention[]>();
    for (const c of conventions) {
      const key = `${c.student.lastName}|${c.student.firstName}|${c.student.className}`;
      const list = map.get(key) || [];
      list.push(c);
      map.set(key, list);
    }
    return [...map.entries()];
  }, [conventions]);

  return (
    <div className="grid gap-8 lg:grid-cols-2">
      <div
        className="rounded-2xl border border-emerald-100 bg-emerald-50/40 p-6 shadow-sm space-y-3 text-sm"
        data-tour="stages-deposer-link"
      >
        <h2 className="text-lg font-bold text-[#1F3D2B]">Dépôt élève (PDF)</h2>
        <p className="text-stone-600">
          Les élèves envoient leur convention remplie et signée sur la page publique. Plus besoin de
          remplir une préconvention en ligne.
        </p>
        <a
          href="/stages/deposer"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-block rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white"
        >
          Ouvrir /stages/deposer →
        </a>
      </div>

      <div data-tour="stages-conventions" className="space-y-6 lg:col-span-2">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Dossiers élèves</h2>
        {dossiers.map(([key, list]) => {
          const first = list[0]!;
          return (
            <div key={key} className="rounded-xl border border-stone-200 bg-white p-4">
              <p className="font-semibold">
                {first.student.firstName} {first.student.lastName} — {first.student.className}
              </p>
              <p className="text-xs text-stone-500">{list.length} convention(s)</p>
              <ul className="mt-2 space-y-2">
                {list.map((c) => (
                  <li key={c.id} className="flex flex-wrap items-center gap-2">
                    <button
                      type="button"
                      className="text-sm text-[#2F6B4A] font-medium underline"
                      onClick={() => onLoadDetail(c.id)}
                    >
                      {c.company.name} · {STAGE_CONVENTION_STATUS_LABELS[c.status]}
                    </button>
                    {permissions?.canFileToOneDrive && oneDriveEnabled && (
                      <>
                        {c.oneDriveFiling?.filedAt ? (
                          <span className="text-xs font-semibold text-emerald-700">OneDrive ✓</span>
                        ) : c.status === "signed" ? (
                          <button
                            type="button"
                            disabled={!oneDriveConnected || filingConventionId === c.id || busy}
                            onClick={() => onFileOneDrive(c.id)}
                            className="rounded border border-[#2F6B4A]/40 px-2 py-0.5 text-xs font-semibold text-[#2F6B4A] disabled:opacity-50"
                          >
                            {filingConventionId === c.id ? "Envoi…" : "→ OneDrive"}
                          </button>
                        ) : null}
                      </>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          );
        })}
      </div>
    </div>
  );
}
