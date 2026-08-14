"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";

export default function SettingsIntegrationsPanel({
  integrations,
  setIntegrations,
  saving,
  saveSection,
}: {
  integrations: Record<string, unknown>;
  setIntegrations: Dispatch<SetStateAction<Record<string, unknown>>>;
  saving: boolean;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  return (
    <div className="bg-white rounded-2xl border p-6 space-y-4">
      <p className="text-sm text-slate-600">
        Paramètres <strong>métier</strong> (pas les raccourcis du tableau de bord — voir l&apos;onglet{" "}
        <strong>Raccourcis tableau de bord</strong>).
      </p>
      <label className="block text-sm font-bold">Zeendoc activé</label>
      <select
        className="w-full border rounded-xl p-3"
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
        className="w-full border rounded-xl p-3"
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
        className="w-full border rounded-xl p-3"
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
          basesBySecteur?: Partial<Record<"ecole" | "college" | "lycee", { basePath?: string; label?: string }>>;
          userSecteurs?: Array<{ match: string; secteur: "ecole" | "college" | "lycee" }>;
        }) || {};
        const bases = od.basesBySecteur || {};
        const userSecteurs = od.userSecteurs || [];
        const setOd = (patch: object) =>
          setIntegrations({
            ...integrations,
            microsoftOneDrive: { ...od, ...patch },
          });
        const setBase = (secteur: "ecole" | "college" | "lycee", basePath: string) =>
          setOd({ basesBySecteur: { ...bases, [secteur]: { ...(bases[secteur] || {}), basePath } } });
        const cycles: Array<{ key: "ecole" | "college" | "lycee"; label: string; placeholder: string }> = [
          { key: "ecole", label: "École", placeholder: "Dossier élèves/École" },
          { key: "college", label: "Collège", placeholder: "Dossier élèves/Collège" },
          { key: "lycee", label: "Lycée", placeholder: "Dossier élèves/Lycée" },
        ];
        return (
          <div className="space-y-4 border-t pt-4">
            <div>
              <p className="text-sm font-bold">Dossiers racine OneDrive par cycle</p>
              <p className="text-xs text-slate-500 mb-2">
                Chemin du dossier (depuis la racine OneDrive) où l&apos;agent IA range les documents
                élèves. Laissez vide pour utiliser la valeur par défaut.
              </p>
              <div className="space-y-2">
                {cycles.map((c) => (
                  <div key={c.key} className="flex items-center gap-2">
                    <span className="w-20 text-sm">{c.label}</span>
                    <input
                      className="flex-1 border rounded-xl p-2 text-sm"
                      placeholder={c.placeholder}
                      value={bases[c.key]?.basePath || ""}
                      onChange={(e) => setBase(c.key, e.target.value)}
                    />
                  </div>
                ))}
              </div>
            </div>

            <div>
              <p className="text-sm font-bold">Comptes → cycle (classement OCR)</p>
              <p className="text-xs text-slate-500 mb-2">
                Associe un e-mail (ou nom de famille) au cycle dont la personne gère le classement.
                Utile pour les secrétariats non câblés en dur.
              </p>
              <div className="space-y-2">
                {userSecteurs.map((row, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <input
                      className="flex-1 border rounded-xl p-2 text-sm"
                      placeholder="email ou nom de famille"
                      value={row.match}
                      onChange={(e) => {
                        const next = [...userSecteurs];
                        next[i] = { ...row, match: e.target.value };
                        setOd({ userSecteurs: next });
                      }}
                    />
                    <select
                      className="border rounded-xl p-2 text-sm"
                      value={row.secteur}
                      onChange={(e) => {
                        const next = [...userSecteurs];
                        next[i] = { ...row, secteur: e.target.value as "ecole" | "college" | "lycee" };
                        setOd({ userSecteurs: next });
                      }}
                    >
                      <option value="ecole">École</option>
                      <option value="college">Collège</option>
                      <option value="lycee">Lycée</option>
                    </select>
                    <button
                      type="button"
                      onClick={() => setOd({ userSecteurs: userSecteurs.filter((_, j) => j !== i) })}
                      className="text-red-600 px-2 text-lg leading-none"
                      aria-label="Supprimer"
                    >
                      ×
                    </button>
                  </div>
                ))}
                <button
                  type="button"
                  onClick={() => setOd({ userSecteurs: [...userSecteurs, { match: "", secteur: "college" }] })}
                  className="text-sm text-indigo-600 font-semibold"
                >
                  + Ajouter un mapping
                </button>
              </div>
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
    </div>
  );
}
