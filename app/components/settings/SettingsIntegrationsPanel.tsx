"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import DirectoryPersonSelect, { directoryMemberLabel } from "@/app/components/settings/DirectoryPersonSelect";
import {
  SettingsNotice,
  SettingsSection,
  settingsInputClass,
  settingsSelectClass,
} from "@/app/components/settings/SettingsChrome";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import {
  OCR_FLUX_IDS,
  OCR_FLUX_META,
  mergeOcrFluxGrid,
  migrateLegacyUserSecteursToOcrFlux,
  type OcrFluxAssignment,
  type OcrFluxId,
} from "@/app/lib/ocr-flux";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

type OneDriveCycle = "ecole" | "college" | "lycee";

type OneDriveUserSecteurRow = {
  externalUserId?: string;
  match: string;
  displayName?: string;
  secteur: OneDriveCycle;
};

function elevesUserSecteursFromFlux(grid: OcrFluxAssignment[]): OneDriveUserSecteurRow[] {
  const out: OneDriveUserSecteurRow[] = [];
  for (const row of grid) {
    const meta = OCR_FLUX_META[row.id];
    if (meta.kind !== "eleves" || !meta.secteur) continue;
    if (!row.externalUserId && !row.match) continue;
    out.push({
      externalUserId: row.externalUserId,
      match: row.match || row.externalUserId || "",
      displayName: row.displayName,
      secteur: meta.secteur,
    });
  }
  return out;
}

function basesBySecteurFromFlux(
  grid: OcrFluxAssignment[],
): Partial<Record<OneDriveCycle, { basePath: string }>> {
  const bases: Partial<Record<OneDriveCycle, { basePath: string }>> = {};
  for (const row of grid) {
    const meta = OCR_FLUX_META[row.id];
    if (meta.kind !== "eleves" || !meta.secteur) continue;
    const path = row.basePath?.trim();
    if (path) bases[meta.secteur] = { basePath: path };
  }
  return bases;
}

export default function SettingsIntegrationsPanel({
  integrations,
  setIntegrations,
  saving,
  saveSection,
  activeEstablishmentKinds: _activeEstablishmentKinds,
  activeCycleLabels,
  directoryMembers,
  membersLoading,
}: {
  integrations: Record<string, unknown>;
  setIntegrations: Dispatch<SetStateAction<Record<string, unknown>>>;
  saving: boolean;
  saveSection: (section: string, body: unknown) => Promise<void>;
  activeEstablishmentKinds?: Set<string>;
  activeCycleLabels?: Partial<Record<OneDriveCycle, string[]>>;
  directoryMembers: DirectoryMemberOption[];
  membersLoading?: boolean;
}) {
  const od = (integrations.microsoftOneDrive as {
    enabled?: boolean;
    basesBySecteur?: Partial<Record<OneDriveCycle, { basePath?: string; label?: string }>>;
    userSecteurs?: OneDriveUserSecteurRow[];
    ocrFlux?: OcrFluxAssignment[];
    rhDrive?: { basePath?: string };
  }) || {};

  const fluxGrid = migrateLegacyUserSecteursToOcrFlux({
    ocrFlux: od.ocrFlux,
    userSecteurs: od.userSecteurs,
    basesBySecteur: od.basesBySecteur,
    personnelBasePath: od.rhDrive?.basePath,
  });

  const setOd = (patch: Record<string, unknown>) =>
    setIntegrations({
      ...integrations,
      microsoftOneDrive: { ...od, ...patch },
    });

  const setFluxGrid = (next: OcrFluxAssignment[]) => {
    const merged = mergeOcrFluxGrid(next);
    setOd({
      ocrFlux: merged,
      userSecteurs: elevesUserSecteursFromFlux(merged),
      basesBySecteur: {
        ...(od.basesBySecteur || {}),
        ...basesBySecteurFromFlux(merged),
      },
    });
  };

  const updateFlux = (id: OcrFluxId, patch: Partial<OcrFluxAssignment>) => {
    setFluxGrid(fluxGrid.map((row) => (row.id === id ? { ...row, ...patch } : row)));
  };

  const cycleLabel = (secteur: Secteur | null) => {
    if (!secteur) return "";
    const names = activeCycleLabels?.[secteur]?.filter(Boolean) ?? [];
    return names[0] || "";
  };

  return (
    <SettingsSection
      icon="🔌"
      title="Intégrations"
      description={
        <>
          Paramètres métier (pas les raccourcis du tableau de bord — voir l&apos;onglet{" "}
          <strong>Raccourcis tableau de bord</strong>).
        </>
      }
    >
      <label className="block text-sm font-bold">Zeendoc activé</label>
      <select
        className={settingsSelectClass}
        value={(integrations.zeendoc as { enabled?: boolean })?.enabled ? "yes" : "no"}
        onChange={(e) =>
          setIntegrations({
            ...integrations,
            zeendoc: {
              ...((integrations.zeendoc as object) || {}),
              enabled: e.target.value === "yes",
              buttonLabel: e.target.value === "yes" ? "Envoyer sur Zeendoc" : "Envoyer par mail",
            },
          })
        }
      >
        <option value="no">Non</option>
        <option value="yes">Oui</option>
      </select>
      <label className="block text-sm font-bold">E-mail destination Zeendoc / envoi PDF</label>
      <input
        className={settingsInputClass}
        value={String((integrations.zeendoc as { destinationEmail?: string })?.destinationEmail || "")}
        onChange={(e) =>
          setIntegrations({
            ...integrations,
            zeendoc: { ...((integrations.zeendoc as object) || {}), destinationEmail: e.target.value },
          })
        }
      />
      <label className="block text-sm font-bold">OneDrive / OCR activé</label>
      <select
        className={settingsSelectClass}
        value={od.enabled ? "yes" : "no"}
        onChange={(e) =>
          setOd({
            enabled: e.target.value === "yes",
          })
        }
      >
        <option value="no">Non</option>
        <option value="yes">Oui</option>
      </select>

      <div className="space-y-4 border-t border-white/60 pt-4">
        <div>
          <p className="text-sm font-bold">Flux OCR → personne</p>
          <p className="text-xs text-slate-500 mb-3">
            Chaque ligne est un flux de classement (élèves, enseignants, personnel OGEC). La même
            personne peut être choisie plusieurs fois (ex. collège + lycée). Les 3 flux enseignants
            partagent par défaut le même chemin OneDrive (« Dossier enseignants ») : les dossiers
            personnes sont fusionnés. Les documents sont rangés selon les flux de la personne
            connectée — sans bouton élève / prof.
          </p>
          {membersLoading ? (
            <p className="text-xs text-slate-500">Chargement du personnel…</p>
          ) : directoryMembers.length === 0 ? (
            <SettingsNotice tone="warn">
              Aucun membre trouvé. Vérifiez l&apos;onglet Utilisateurs ou les invitations.
            </SettingsNotice>
          ) : (
            <div className="space-y-3">
              {OCR_FLUX_IDS.map((id) => {
                const row = fluxGrid.find((r) => r.id === id) ?? { id };
                const meta = OCR_FLUX_META[id];
                const extra = cycleLabel(meta.secteur);
                return (
                  <div
                    key={id}
                    className="rounded-2xl border border-white/70 bg-white/50 p-4 space-y-3"
                  >
                    <p className="text-xs font-bold uppercase tracking-wide text-slate-500">
                      {meta.label}
                      {extra ? ` · ${extra}` : ""}
                    </p>
                    <DirectoryPersonSelect
                      members={directoryMembers}
                      selectedId={row.externalUserId}
                      selectedEmail={row.match}
                      loading={membersLoading}
                      onChange={(member) => {
                        if (!member) {
                          updateFlux(id, {
                            externalUserId: undefined,
                            match: undefined,
                            displayName: undefined,
                          });
                          return;
                        }
                        updateFlux(id, {
                          externalUserId: member.externalUserId,
                          match: member.email.trim(),
                          displayName: directoryMemberLabel(member),
                        });
                      }}
                    />
                    <label className="block text-xs font-semibold text-slate-600">
                      Dossier racine OneDrive
                      <input
                        className={`${settingsInputClass} mt-1`}
                        placeholder={meta.defaultBasePath}
                        value={row.basePath || ""}
                        onChange={(e) => updateFlux(id, { basePath: e.target.value })}
                      />
                    </label>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      <ModuleButton
        variant="primary"
        disabled={saving}
        onClick={() => {
          const merged = mergeOcrFluxGrid(fluxGrid);
          void saveSection("integrations", {
            ...integrations,
            microsoftOneDrive: {
              ...od,
              ocrFlux: merged,
              userSecteurs: elevesUserSecteursFromFlux(merged),
              basesBySecteur: {
                ...(od.basesBySecteur || {}),
                ...basesBySecteurFromFlux(merged),
              },
            },
          });
        }}
      >
        Enregistrer intégrations
      </ModuleButton>
    </SettingsSection>
  );
}
