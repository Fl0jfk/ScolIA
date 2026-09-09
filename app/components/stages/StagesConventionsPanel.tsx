"use client";

import { useMemo, type ReactNode } from "react";
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
  selectedId,
  detailPanel,
  onLoadDetail,
  onCloseDetail,
  onFileOneDrive,
}: {
  conventions: StageConvention[];
  permissions: StagesHubPermissions | undefined;
  oneDriveEnabled: boolean;
  oneDriveConnected: boolean;
  filingConventionId: string | null;
  busy: boolean;
  selectedId: string | null;
  detailPanel: ReactNode;
  onLoadDetail: (id: string) => void;
  onCloseDetail: () => void;
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
        data-tour="stages-preconvention-link"
      >
        <h2 className="text-lg font-bold text-[#1F3D2B]">Formulaire public élèves</h2>
        <p className="text-stone-600">
          Lien à communiquer aux familles : identification INE puis formulaire en ligne (entreprise,
          horaires, dates) — pas de dépôt PDF.
        </p>
        <a
          href="/stages/preconvention"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-block rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white"
        >
          Ouvrir la page élève →
        </a>
      </div>

      <div data-tour="stages-conventions" className="space-y-4 lg:col-span-2">
        <h2 className="text-lg font-bold text-[#1F3D2B]">Dossiers élèves</h2>
        {dossiers.map(([key, list]) => {
          const first = list[0]!;
          const openHere = list.some((c) => c.id === selectedId);
          return (
            <div
              key={key}
              className={`rounded-xl border bg-white p-4 transition-shadow ${
                openHere
                  ? "border-[#2F6B4A]/40 shadow-md ring-1 ring-[#2F6B4A]/15"
                  : "border-stone-200 shadow-sm"
              }`}
            >
              <button
                type="button"
                className="flex w-full items-center justify-between gap-3 text-left"
                onClick={() => {
                  if (openHere) {
                    onCloseDetail();
                    return;
                  }
                  const prefer =
                    list.find((c) => c.status === "signatures_pending" || c.status === "admin_review") ||
                    list[0];
                  if (prefer) onLoadDetail(prefer.id);
                }}
                aria-expanded={openHere}
              >
                <div>
                  <p className="font-semibold text-[#1F3D2B]">
                    {first.student.firstName} {first.student.lastName} — {first.student.className}
                  </p>
                  <p className="text-xs text-stone-500">{list.length} convention(s)</p>
                </div>
                <span className="text-xs font-semibold text-[#2F6B4A]">
                  {openHere ? "Réduire ▲" : "Ouvrir ▼"}
                </span>
              </button>

              <ul className="mt-3 space-y-2">
                {list.map((c) => {
                  const active = c.id === selectedId;
                  return (
                    <li key={c.id}>
                      <div className="flex flex-wrap items-center gap-2">
                        <button
                          type="button"
                          className={`rounded-lg px-2 py-1 text-sm font-medium ${
                            active
                              ? "bg-[#2F6B4A] text-white"
                              : "text-[#2F6B4A] underline hover:bg-emerald-50"
                          }`}
                          onClick={() => {
                            if (active) onCloseDetail();
                            else onLoadDetail(c.id);
                          }}
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
                      </div>
                      {active && detailPanel}
                    </li>
                  );
                })}
              </ul>
            </div>
          );
        })}
      </div>
    </div>
  );
}
