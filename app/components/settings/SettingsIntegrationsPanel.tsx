"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ClerkPersonSelect, { clerkMemberLabel } from "@/app/components/settings/ClerkPersonSelect";
import {
  SettingsNotice,
  SettingsSection,
  settingsInputClass,
  settingsSelectClass,
} from "@/app/components/settings/SettingsChrome";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import { DEFAULT_ONEDRIVE_BASE_BY_SECTEUR } from "@/app/lib/onedrive-user-profiles";

type OneDriveCycle = "ecole" | "college" | "lycee";

type OneDriveUserSecteurRow = {
  clerkUserId?: string;
  match: string;
  displayName?: string;
  secteur: OneDriveCycle;
};

const ALL_ONEDRIVE_CYCLES: Array<{ key: OneDriveCycle; label: string }> = [
  { key: "ecole", label: "École" },
  { key: "college", label: "Collège" },
  { key: "lycee", label: "Lycée" },
];

export default function SettingsIntegrationsPanel({
  integrations,
  setIntegrations,
  saving,
  saveSection,
  activeEstablishmentKinds,
  activeCycleLabels,
  clerkMembers,
  membersLoading,
}: {
  integrations: Record<string, unknown>;
  setIntegrations: Dispatch<SetStateAction<Record<string, unknown>>>;
  saving: boolean;
  saveSection: (section: string, body: unknown) => Promise<void>;
  activeEstablishmentKinds?: Set<string>;
  activeCycleLabels?: Partial<Record<OneDriveCycle, string[]>>;
  clerkMembers: ClerkMemberOption[];
  membersLoading?: boolean;
}) {
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
        value={(integrations.microsoftOneDrive as { enabled?: boolean })?.enabled ? "yes" : "no"}
        onChange={(e) =>
          setIntegrations({
            ...integrations,
            microsoftOneDrive: {
              ...((integrations.microsoftOneDrive as object) || {}),
              enabled: e.target.value === "yes",
            },
          })
        }
      >
        <option value="no">Non</option>
        <option value="yes">Oui</option>
      </select>

      {(() => {
        const od = (integrations.microsoftOneDrive as {
          enabled?: boolean;
          basesBySecteur?: Partial<Record<OneDriveCycle, { basePath?: string; label?: string }>>;
          userSecteurs?: OneDriveUserSecteurRow[];
        }) || {};
        const bases = od.basesBySecteur || {};
        const userSecteurs = od.userSecteurs || [];
        const setOd = (patch: object) =>
          setIntegrations({
            ...integrations,
            microsoftOneDrive: { ...od, ...patch },
          });
        const setBase = (secteur: OneDriveCycle, basePath: string) =>
          setOd({ basesBySecteur: { ...bases, [secteur]: { ...(bases[secteur] || {}), basePath } } });
        const updateUserSecteur = (index: number, patch: Partial<OneDriveUserSecteurRow>) => {
          const next = userSecteurs.map((row, i) => (i === index ? { ...row, ...patch } : row));
          setOd({ userSecteurs: next });
        };
        const removeUserSecteur = (index: number) => {
          setOd({ userSecteurs: userSecteurs.filter((_, j) => j !== index) });
        };
        const assignedClerkIds = new Set(
          userSecteurs.map((r) => r.clerkUserId?.trim()).filter(Boolean) as string[],
        );
        const cycles = activeEstablishmentKinds
          ? ALL_ONEDRIVE_CYCLES.filter((c) => activeEstablishmentKinds.has(c.key))
          : ALL_ONEDRIVE_CYCLES;
        const defaultSecteur = cycles[0]?.key ?? "college";
        return (
          <div className="space-y-4 border-t border-white/60 pt-4">
            <div>
              <p className="text-sm font-bold">Dossiers racine OneDrive par cycle</p>
              <p className="text-xs text-slate-500 mb-2">
                Chemin du dossier (depuis la racine OneDrive) où l&apos;agent IA range les documents
                élèves. Un champ par établissement actif de type école, collège ou lycée. Laissez vide
                pour utiliser la valeur par défaut.
              </p>
              {cycles.length === 0 ? (
                <SettingsNotice tone="warn">
                  Aucun établissement école, collège ou lycée actif. Ajoutez un site dans l&apos;onglet
                  Sites / directions pour configurer les dossiers OneDrive correspondants.
                </SettingsNotice>
              ) : (
                <div className="space-y-2">
                  {cycles.map((c) => {
                    const names = activeCycleLabels?.[c.key]?.filter(Boolean) ?? [];
                    const label = names.length ? names.join(" · ") : c.label;
                    return (
                      <div key={c.key} className="flex items-center gap-2">
                        <span className="w-36 shrink-0 text-sm font-medium" title={label}>
                          {label}
                        </span>
                        <input
                          className={`flex-1 ${settingsInputClass} mt-0`}
                          placeholder={DEFAULT_ONEDRIVE_BASE_BY_SECTEUR[c.key].basePath}
                          value={bases[c.key]?.basePath || ""}
                          onChange={(e) => setBase(c.key, e.target.value)}
                        />
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            <div>
              <p className="text-sm font-bold">Comptes → cycle (classement OCR)</p>
              <p className="text-xs text-slate-500 mb-2">
                Choisissez une personne du personnel Clerk et le cycle (école, collège ou lycée) dont elle
                gère le classement OneDrive. Chaque personne ne peut être assignée qu&apos;à un seul cycle.
              </p>
              {cycles.length === 0 ? (
                <p className="text-xs text-slate-500">Disponible dès qu&apos;un cycle école / collège / lycée est actif.</p>
              ) : membersLoading ? (
                <p className="text-xs text-slate-500">Chargement du personnel Clerk…</p>
              ) : clerkMembers.length === 0 ? (
                <SettingsNotice tone="warn">
                  Aucun membre Clerk trouvé. Vérifiez l&apos;onglet Utilisateurs ou les invitations Clerk.
                </SettingsNotice>
              ) : (
                <div className="space-y-4">
                  {userSecteurs.map((row, i) => (
                    <div
                      key={row.clerkUserId || `row-${i}`}
                      className="rounded-2xl border border-white/70 bg-white/50 p-4 space-y-3"
                    >
                      <div className="flex items-start justify-between gap-2">
                        <p className="text-xs font-bold uppercase tracking-wide text-slate-500">
                          Responsable classement {i + 1}
                        </p>
                        <button
                          type="button"
                          onClick={() => removeUserSecteur(i)}
                          className="text-red-600 px-1 text-lg leading-none shrink-0"
                          aria-label="Supprimer"
                        >
                          ×
                        </button>
                      </div>
                      <ClerkPersonSelect
                        members={clerkMembers.filter(
                          (m) =>
                            m.clerkUserId === row.clerkUserId ||
                            !assignedClerkIds.has(m.clerkUserId),
                        )}
                        selectedId={row.clerkUserId}
                        selectedEmail={row.match}
                        loading={membersLoading}
                        onChange={(member) => {
                          if (!member) {
                            removeUserSecteur(i);
                            return;
                          }
                          updateUserSecteur(i, {
                            clerkUserId: member.clerkUserId,
                            match: member.email.trim(),
                            displayName: clerkMemberLabel(member),
                          });
                        }}
                      />
                      <label className="block text-xs font-semibold text-slate-600">
                        Cycle géré
                        <select
                          className={`${settingsSelectClass} mt-1`}
                          value={cycles.some((c) => c.key === row.secteur) ? row.secteur : defaultSecteur}
                          onChange={(e) =>
                            updateUserSecteur(i, { secteur: e.target.value as OneDriveCycle })
                          }
                        >
                          {cycles.map((c) => (
                            <option key={c.key} value={c.key}>
                              {activeCycleLabels?.[c.key]?.[0] || c.label}
                            </option>
                          ))}
                        </select>
                      </label>
                    </div>
                  ))}
                  <button
                    type="button"
                    onClick={() =>
                      setOd({
                        userSecteurs: [...userSecteurs, { match: "", secteur: defaultSecteur }],
                      })
                    }
                    className="text-sm font-semibold text-[var(--dash-primary)]"
                  >
                    + Ajouter une personne
                  </button>
                  {userSecteurs.length === 0 && (
                    <p className="text-xs text-slate-500 italic">
                      Aucune personne assignée — le classement OCR ne fonctionnera que pour les comptes déjà
                      configurés ailleurs.
                    </p>
                  )}
                </div>
              )}
            </div>
          </div>
        );
      })()}

      <ModuleButton
        variant="primary"
        disabled={saving}
        onClick={() => saveSection("integrations", integrations)}
      >
        Enregistrer intégrations
      </ModuleButton>
    </SettingsSection>
  );
}
