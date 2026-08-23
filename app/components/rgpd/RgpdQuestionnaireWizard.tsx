"use client";

import { useEffect, useState } from "react";
import {
  RGPD_CATALOG,
  RGPD_QUESTIONNAIRE_STEPS,
  RGPD_TOTAL_STEPS,
  evaluateAllDocumentRequirements,
} from "@/app/lib/rgpd-catalog";
import { RGPD_PLATFORM_DPAS } from "@/app/lib/rgpd-platform-dpas";
import { RGPD_ENT_PRESETS, applyRgpdEntPreset } from "@/app/lib/rgpd-ent-presets";
import type { RgpdEntPresetId, RgpdQuestionnaireAnswers } from "@/app/lib/rgpd-types";

const inputClass =
  "w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm outline-none focus:border-indigo-400";

type DirectoryUserOption = {
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  roles: string[];
};

function directoryUserLabel(u: DirectoryUserOption): string {
  return u.displayName || `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim() || u.email;
}

type Props = {
  answers: RgpdQuestionnaireAnswers;
  onChange: (patch: Partial<RgpdQuestionnaireAnswers>) => void;
  onSave: (patch: Partial<RgpdQuestionnaireAnswers>, complete?: boolean) => Promise<void>;
  saving: boolean;
};

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-sm text-slate-700 cursor-pointer">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      {label}
    </label>
  );
}

export default function RgpdQuestionnaireWizard({ answers, onChange, onSave, saving }: Props) {
  const step = Math.min(answers.questionnaireStep, RGPD_TOTAL_STEPS);
  const [directoryUsers, setDirectoryUsers] = useState<DirectoryUserOption[]>([]);
  const [directoryUsersLoading, setDirectoryUsersLoading] = useState(false);
  const [directoryUsersError, setDirectoryUsersError] = useState<string | null>(null);

  useEffect(() => {
    if (step !== 2 || !answers.dpdDesignated || !answers.dpdInternal) return;
    let cancelled = false;
    setDirectoryUsersLoading(true);
    setDirectoryUsersError(null);
    fetch("/api/rgpd/directory-users", { cache: "no-store" })
      .then(async (res) => {
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || "Chargement impossible");
        if (!cancelled) setDirectoryUsers(data.users ?? []);
      })
      .catch((e) => {
        if (!cancelled) {
          setDirectoryUsersError(e instanceof Error ? e.message : String(e));
          setDirectoryUsers([]);
        }
      })
      .finally(() => {
        if (!cancelled) setDirectoryUsersLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [step, answers.dpdDesignated, answers.dpdInternal]);

  const go = async (next: number, patch?: Partial<RgpdQuestionnaireAnswers>) => {
    if (patch) onChange(patch);
    onChange({ questionnaireStep: next });
    await onSave({ ...patch, questionnaireStep: next }, next >= RGPD_TOTAL_STEPS);
  };

  const requirements = evaluateAllDocumentRequirements(answers);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap gap-2">
        {RGPD_QUESTIONNAIRE_STEPS.map((s) => (
          <span
            key={s.id}
            className={`text-xs px-2 py-1 rounded-full border ${
              s.id === step
                ? "bg-indigo-600 text-white border-indigo-600"
                : s.id < step
                  ? "bg-emerald-50 text-emerald-800 border-emerald-200"
                  : "bg-slate-50 text-slate-500 border-slate-200"
            }`}
          >
            {s.id}. {s.title}
          </span>
        ))}
      </div>

      <div className="rounded-2xl border bg-white p-6">
        {step === 1 && (
          <div className="space-y-4">
            <p className="text-sm font-bold text-slate-800">Profil de l&apos;établissement</p>
            <p className="text-xs text-slate-500">
              Cochez les cycles présents. Pour un groupe scolaire (école + collège + lycée), sélectionnez
              plusieurs niveaux ci-dessous.
            </p>
            <div className="flex flex-wrap gap-3">
              {(
                [
                  ["ecole", "École"],
                  ["college", "Collège"],
                  ["lycee", "Lycée"],
                ] as const
              ).map(([k, label]) => (
                <Toggle
                  key={k}
                  label={label}
                  checked={answers.establishmentKinds.includes(k)}
                  onChange={() => {
                    const kinds = answers.establishmentKinds.includes(k)
                      ? answers.establishmentKinds.filter((x) => x !== k)
                      : [...answers.establishmentKinds, k];
                    onChange({
                      establishmentKinds: kinds,
                      isGroup: kinds.length >= 2,
                    });
                  }}
                />
              ))}
            </div>
            <div className="grid sm:grid-cols-3 gap-3">
              <label className="text-sm">
                Effectif élèves (approx.)
                <input
                  type="number"
                  className={inputClass + " mt-1"}
                  value={answers.studentCount ?? ""}
                  onChange={(e) =>
                    onChange({
                      studentCount: e.target.value ? Number(e.target.value) : undefined,
                    })
                  }
                />
              </label>
              <label className="text-sm">
                Effectif professeurs
                <input
                  type="number"
                  className={inputClass + " mt-1"}
                  value={answers.teacherCount ?? ""}
                  onChange={(e) =>
                    onChange({
                      teacherCount: e.target.value ? Number(e.target.value) : undefined,
                    })
                  }
                />
              </label>
              <label className="text-sm">
                Effectif personnel (hors enseignants)
                <input
                  type="number"
                  className={inputClass + " mt-1"}
                  value={answers.staffCount ?? ""}
                  onChange={(e) =>
                    onChange({
                      staffCount: e.target.value ? Number(e.target.value) : undefined,
                    })
                  }
                />
              </label>
            </div>

            <div className="border-t pt-4 space-y-3">
              <p className="text-sm font-bold text-slate-800">Fiche 0 — Identité (registre)</p>
              <label className="text-sm block">
                Nom de l&apos;établissement / groupe scolaire
                <input
                  className={inputClass + " mt-1"}
                  placeholder="Ex. La Providence Nicolas Barré"
                  value={answers.establishmentIdentity?.legalName ?? ""}
                  onChange={(e) =>
                    onChange({
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        legalName: e.target.value,
                      },
                    })
                  }
                />
              </label>
              <label className="text-sm block">
                Type d&apos;établissement
                <input
                  className={inputClass + " mt-1"}
                  placeholder="Ex. Établissement privé catholique sous contrat"
                  value={answers.establishmentIdentity?.establishmentType ?? ""}
                  onChange={(e) =>
                    onChange({
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        establishmentType: e.target.value,
                      },
                    })
                  }
                />
              </label>
              <label className="text-sm block">
                Niveaux accueillis (détail)
                <textarea
                  className={inputClass + " mt-1 min-h-[72px]"}
                  placeholder="Maternelle, primaire cycles 1-3, collège, lycée général/technologique…"
                  value={answers.establishmentIdentity?.levelsDescription ?? ""}
                  onChange={(e) =>
                    onChange({
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        levelsDescription: e.target.value,
                      },
                    })
                  }
                />
              </label>
              <div className="grid sm:grid-cols-2 gap-3">
                <label className="text-sm block">
                  Coordinateur RGPD — nom
                  <input
                    className={inputClass + " mt-1"}
                    value={answers.establishmentIdentity?.coordinatorName ?? ""}
                    onChange={(e) =>
                      onChange({
                        establishmentIdentity: {
                          ...answers.establishmentIdentity,
                          coordinatorName: e.target.value,
                        },
                        directionReferent: e.target.value || answers.directionReferent,
                      })
                    }
                  />
                </label>
                <label className="text-sm block">
                  Titre
                  <input
                    className={inputClass + " mt-1"}
                    placeholder="Chef d'établissement"
                    value={answers.establishmentIdentity?.coordinatorTitle ?? ""}
                    onChange={(e) =>
                      onChange({
                        establishmentIdentity: {
                          ...answers.establishmentIdentity,
                          coordinatorTitle: e.target.value,
                        },
                      })
                    }
                  />
                </label>
                <label className="text-sm block">
                  E-mail coordinateur
                  <input
                    type="email"
                    className={inputClass + " mt-1"}
                    value={answers.establishmentIdentity?.coordinatorEmail ?? ""}
                    onChange={(e) =>
                      onChange({
                        establishmentIdentity: {
                          ...answers.establishmentIdentity,
                          coordinatorEmail: e.target.value,
                        },
                      })
                    }
                  />
                </label>
                <label className="text-sm block">
                  Téléphone
                  <input
                    className={inputClass + " mt-1"}
                    value={answers.establishmentIdentity?.coordinatorPhone ?? ""}
                    onChange={(e) =>
                      onChange({
                        establishmentIdentity: {
                          ...answers.establishmentIdentity,
                          coordinatorPhone: e.target.value,
                        },
                      })
                    }
                  />
                </label>
              </div>
              <div className="grid sm:grid-cols-2 gap-3">
                <label className="text-sm block">
                  DPO désigné par
                  <input
                    className={inputClass + " mt-1"}
                    placeholder="Réseau diocésain / FNOGEC…"
                    value={answers.establishmentIdentity?.dpoDesignatedBy ?? ""}
                    onChange={(e) =>
                      onChange({
                        establishmentIdentity: {
                          ...answers.establishmentIdentity,
                          dpoDesignatedBy: e.target.value,
                        },
                      })
                    }
                  />
                </label>
                <label className="text-sm block">
                  E-mail DPO externe
                  <input
                    className={inputClass + " mt-1"}
                    placeholder="dpd@enseignement-catholique.fr"
                    value={answers.establishmentIdentity?.dpoExternalEmail ?? ""}
                    onChange={(e) =>
                      onChange({
                        establishmentIdentity: {
                          ...answers.establishmentIdentity,
                          dpoExternalEmail: e.target.value,
                        },
                      })
                    }
                  />
                </label>
              </div>
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="space-y-4">
            <p className="text-sm font-bold text-slate-800">Gouvernance RGPD</p>
            <Toggle
              label="DPD / DPO désigné"
              checked={answers.dpdDesignated}
              onChange={(v) => onChange({ ...answers, dpdDesignated: v })}
            />
            {answers.dpdDesignated && (
              <>
                <Toggle
                  label="DPD interne à l'établissement"
                  checked={answers.dpdInternal}
                  onChange={(v) =>
                    onChange({
                      dpdInternal: v,
                      ...(v
                        ? {}
                        : { dpdExternalUserId: undefined, dpdEmail: undefined }),
                    })
                  }
                />
                {answers.dpdInternal ? (
                  <label className="text-sm block">
                    DPD — membre de l&apos;établissement
                    <select
                      className={inputClass + " mt-1"}
                      value={answers.dpdExternalUserId ?? ""}
                      disabled={directoryUsersLoading}
                      onChange={(e) => {
                        const id = e.target.value;
                        const user = directoryUsers.find((u) => u.externalUserId === id);
                        onChange({
                          dpdExternalUserId: id || undefined,
                          dpdName: user ? directoryUserLabel(user) : undefined,
                          dpdEmail: user?.email,
                        });
                      }}
                    >
                      <option value="">
                        {directoryUsersLoading ? "Chargement…" : "— Choisir un utilisateur —"}
                      </option>
                      {directoryUsers.map((u) => (
                        <option key={u.externalUserId} value={u.externalUserId}>
                          {directoryUserLabel(u)}
                          {u.email ? ` (${u.email})` : ""}
                        </option>
                      ))}
                    </select>
                    {directoryUsersError && (
                      <p className="text-xs text-red-600 mt-1">{directoryUsersError}</p>
                    )}
                    {!directoryUsersLoading && directoryUsers.length === 0 && !directoryUsersError && (
                      <p className="text-xs text-slate-500 mt-1">
                        Aucun utilisateur actif trouvé.
                      </p>
                    )}
                  </label>
                ) : (
                  <label className="text-sm block">
                    Nom du DPD (externe)
                    <input
                      className={inputClass + " mt-1"}
                      value={answers.dpdName ?? ""}
                      placeholder="Cabinet ou personne externe"
                      onChange={(e) =>
                        onChange({
                          dpdName: e.target.value,
                          dpdExternalUserId: undefined,
                          dpdEmail: undefined,
                        })
                      }
                    />
                  </label>
                )}
              </>
            )}
            <label className="text-sm block">
              Référent direction
              <input
                className={inputClass + " mt-1"}
                value={answers.directionReferent ?? ""}
                onChange={(e) => onChange({ ...answers, directionReferent: e.target.value })}
              />
            </label>
          </div>
        )}

        {step === 3 && (
          <div className="space-y-3">
            <p className="text-sm font-bold text-slate-800">Publics concernés</p>
            <Toggle
              label="Élèves"
              checked={answers.audiences.students}
              onChange={(v) =>
                onChange({ ...answers, audiences: { ...answers.audiences, students: v } })
              }
            />
            <Toggle
              label="Parents"
              checked={answers.audiences.parents}
              onChange={(v) =>
                onChange({ ...answers, audiences: { ...answers.audiences, parents: v } })
              }
            />
            <Toggle
              label="Personnel"
              checked={answers.audiences.staff}
              onChange={(v) =>
                onChange({ ...answers, audiences: { ...answers.audiences, staff: v } })
              }
            />
            <Toggle
              label="Prospects (portes ouvertes, pré-inscriptions)"
              checked={answers.audiences.prospects}
              onChange={(v) =>
                onChange({ ...answers, audiences: { ...answers.audiences, prospects: v } })
              }
            />
            <Toggle
              label="Anciens élèves"
              checked={answers.audiences.alumni}
              onChange={(v) =>
                onChange({ ...answers, audiences: { ...answers.audiences, alumni: v } })
              }
            />
          </div>
        )}

        {step === 4 && (
          <div className="space-y-3">
            <p className="text-sm font-bold text-slate-800">Traitements sensibles ou à risque</p>
            {(
              [
                ["photosVideos", "Photos / vidéos"],
                ["publications", "Publications (site, réseaux)"],
                ["boarding", "Internat"],
                ["healthData", "Données de santé"],
                ["videoSurveillance", "Vidéosurveillance"],
                ["biometrics", "Biométrie"],
                ["aiTools", "Outils d'IA / profilage"],
              ] as const
            ).map(([key, label]) => (
              <Toggle
                key={key}
                label={label}
                checked={answers.sensitiveProcessing[key]}
                onChange={(v) =>
                  onChange({
                    ...answers,
                    sensitiveProcessing: { ...answers.sensitiveProcessing, [key]: v },
                  })
                }
              />
            ))}
          </div>
        )}

        {step === 5 && (
          <div className="space-y-3">
            <p className="text-sm font-bold text-slate-800">Outils et sous-traitants</p>

            <div className="rounded-xl border border-indigo-100 bg-indigo-50/40 p-4 space-y-3">
              <p className="text-sm font-bold text-slate-800">
                Fiche 1 — Logiciel / ENT de gestion scolaire
              </p>
              <p className="text-xs text-slate-600">
                Choisissez un preset ou saisissez un autre logiciel. Vous pouvez indiquer un second
                outil (ex. Charlemagne <strong>et</strong> ÉcoleDirecte).
              </p>
              <label className="text-sm block">
                Type de logiciel
                <select
                  className={inputClass + " mt-1"}
                  value={answers.establishmentIdentity?.entPreset ?? "autre"}
                  onChange={(e) => {
                    const preset = e.target.value as RgpdEntPresetId;
                    const applied = applyRgpdEntPreset(preset);
                    onChange({
                      subprocessors: {
                        ...answers.subprocessors,
                        ent: true,
                        entName: applied.entName,
                      },
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        entPreset: preset,
                        entProducts: applied.entProducts,
                        entEditor: applied.entEditor,
                      },
                      processingActivities: {
                        ...answers.processingActivities,
                        adminEnt: true,
                      },
                    });
                  }}
                >
                  {(
                    Object.entries(RGPD_ENT_PRESETS) as [
                      RgpdEntPresetId,
                      (typeof RGPD_ENT_PRESETS)[RgpdEntPresetId],
                    ][]
                  ).map(([id, p]) => (
                    <option key={id} value={id}>
                      {p.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="text-sm block">
                Logiciel / ENT principal
                <input
                  className={inputClass + " mt-1"}
                  placeholder="Ex. Pronote, Charlemagne, ÉcoleDirecte…"
                  value={
                    answers.establishmentIdentity?.entProducts ??
                    answers.subprocessors.entName ??
                    ""
                  }
                  onChange={(e) =>
                    onChange({
                      subprocessors: { ...answers.subprocessors, ent: true, entName: e.target.value },
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        entPreset: "autre",
                        entProducts: e.target.value,
                      },
                      processingActivities: {
                        ...answers.processingActivities,
                        adminEnt: true,
                      },
                    })
                  }
                />
              </label>
              <label className="text-sm block">
                Autre logiciel ou ENT (complément, optionnel)
                <input
                  className={inputClass + " mt-1"}
                  placeholder="Ex. second logiciel si vous en utilisez deux"
                  value={answers.establishmentIdentity?.secondaryEntProducts ?? ""}
                  onChange={(e) =>
                    onChange({
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        secondaryEntProducts: e.target.value,
                      },
                    })
                  }
                />
              </label>
              <label className="text-sm block">
                Éditeur / hébergeur
                <input
                  className={inputClass + " mt-1"}
                  placeholder="Ex. Aplim, Index Éducation, autre…"
                  value={answers.establishmentIdentity?.entEditor ?? ""}
                  onChange={(e) =>
                    onChange({
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        entEditor: e.target.value,
                      },
                    })
                  }
                />
              </label>
            </div>

            <Toggle
              label="ENT / logiciel déclaré comme sous-traitant (liste art. 28)"
              checked={answers.subprocessors.ent}
              onChange={(v) =>
                onChange({
                  subprocessors: { ...answers.subprocessors, ent: v },
                })
              }
            />
            <Toggle
              label="Microsoft 365 (licences)"
              checked={answers.subprocessors.microsoft365}
              onChange={(v) =>
                onChange({
                  subprocessors: { ...answers.subprocessors, microsoft365: v },
                  platformDpas: { ...answers.platformDpas, microsoft365: v },
                })
              }
            />
            <Toggle
              label="Scola (intranet)"
              checked={answers.subprocessors.scola}
              onChange={(v) =>
                onChange({
                  subprocessors: { ...answers.subprocessors, scola: v },
                })
              }
            />
            <Toggle
              label="AWS (hébergement cloud / S3)"
              checked={answers.subprocessors.aws}
              onChange={(v) =>
                onChange({
                  subprocessors: { ...answers.subprocessors, aws: v },
                  platformDpas: { ...answers.platformDpas, aws: v },
                })
              }
            />
            <Toggle
              label="Mistral AI (OCR, analyse, incidents)"
              checked={answers.subprocessors.mistralAi}
              onChange={(v) =>
                onChange({
                  subprocessors: { ...answers.subprocessors, mistralAi: v },
                  platformDpas: { ...answers.platformDpas, mistral: v },
                })
              }
            />
            <Toggle
              label="Autres SaaS"
              checked={answers.subprocessors.otherSaas}
              onChange={(v) =>
                onChange({
                  ...answers,
                  subprocessors: { ...answers.subprocessors, otherSaas: v },
                })
              }
            />
            {answers.subprocessors.otherSaas && (
              <textarea
                className={inputClass + " min-h-[80px]"}
                placeholder="Listez les autres outils"
                value={answers.subprocessors.otherSaasList ?? ""}
                onChange={(e) =>
                  onChange({
                    subprocessors: {
                      ...answers.subprocessors,
                      otherSaasList: e.target.value,
                    },
                  })
                }
              />
            )}

            <div className="rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-2">
              <p className="text-sm font-bold text-slate-800">
                Fiches de traitement du registre (art. 30)
              </p>
              <p className="text-xs text-slate-500">
                Chaque fiche détaillée (finalités, bases légales, durées, sécurité…) est générée
                dans la bibliothèque de documents.
              </p>
              {(
                [
                  ["adminEnt", "Fiche 1 — Gestion administrative (ENT / logiciel scolaire)"],
                  ["messaging", "Fiche 2 — Messagerie Outlook & ENT"],
                  ["paperFiles", "Fiche 3 — Dossiers papier secrétariats"],
                  ["onedriveAi", "Fiche 4 — OneDrive & classement IA"],
                  ["healthData", "Fiche 5 — Données de santé"],
                  ["disciplinary", "Fiche 6 — Dossiers disciplinaires"],
                  ["historicalArchives", "Fiche 7 — Archives historiques"],
                  ["financial", "Fiche 8 — Gestion financière"],
                ] as const
              ).map(([key, label]) => (
                <Toggle
                  key={key}
                  label={label}
                  checked={answers.processingActivities[key]}
                  onChange={(v) =>
                    onChange({
                      processingActivities: {
                        ...answers.processingActivities,
                        [key]: v,
                      },
                      ...(key === "healthData"
                        ? {
                            sensitiveProcessing: {
                              ...answers.sensitiveProcessing,
                              healthData: v,
                            },
                          }
                        : {}),
                    })
                  }
                />
              ))}
              <label className="text-sm block pt-2">
                Jours de conservation zones temporaires IA
                <input
                  type="number"
                  min={1}
                  max={30}
                  className={inputClass + " mt-1 w-32"}
                  value={answers.establishmentIdentity?.tempRetentionDays ?? 7}
                  onChange={(e) =>
                    onChange({
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        tempRetentionDays: Number(e.target.value) || 7,
                      },
                    })
                  }
                />
              </label>
              <label className="text-sm block">
                Archives départementales (contact)
                <input
                  className={inputClass + " mt-1"}
                  value={answers.establishmentIdentity?.archivesDepartmentContact ?? ""}
                  onChange={(e) =>
                    onChange({
                      establishmentIdentity: {
                        ...answers.establishmentIdentity,
                        archivesDepartmentContact: e.target.value,
                      },
                    })
                  }
                />
              </label>
            </div>

            <div className="mt-4 rounded-xl border border-indigo-100 bg-indigo-50/50 p-4 space-y-3">
              <p className="text-sm font-bold text-slate-800">
                Accords de sous-traitance (DPA)
              </p>
              <p className="text-xs text-slate-600">
                Plateformes pré-renseignées pour Scola et l&apos;établissement. Cochez celles
                effectivement utilisées ; les liens DPA seront repris dans la liste des
                sous-traitants.
              </p>
              <div className="space-y-2">
                {RGPD_PLATFORM_DPAS.map((platform) => {
                  const enabled =
                    answers.platformDpas?.[platform.id] ?? platform.defaultEnabled;
                  return (
                    <label
                      key={platform.id}
                      className="flex gap-3 rounded-lg border border-white bg-white/80 p-3 cursor-pointer"
                    >
                      <input
                        type="checkbox"
                        className="mt-1"
                        checked={enabled}
                        onChange={(e) =>
                          onChange({
                            platformDpas: {
                              ...answers.platformDpas,
                              [platform.id]: e.target.checked,
                            },
                          })
                        }
                      />
                      <span className="text-sm flex-1 min-w-0">
                        <span className="font-semibold text-slate-800">{platform.name}</span>
                        <span className="block text-xs text-slate-600 mt-0.5">
                          {platform.purposes}
                        </span>
                        <span className="block text-xs text-slate-500 mt-1">
                          {platform.dataLocation} —{" "}
                          <a
                            href={platform.dpaUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-indigo-600 underline"
                            onClick={(e) => e.stopPropagation()}
                          >
                            {platform.dpaLabel}
                          </a>
                        </span>
                      </span>
                    </label>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {step === 6 && (
          <div className="space-y-3">
            <p className="text-sm font-bold text-slate-800">Mesures déjà en place</p>
            <Toggle
              label="Registre des traitements existant"
              checked={answers.existingMeasures.hasRegister}
              onChange={(v) =>
                onChange({
                  ...answers,
                  existingMeasures: { ...answers.existingMeasures, hasRegister: v },
                })
              }
            />
            <Toggle
              label="Procédure violation existante"
              checked={answers.existingMeasures.hasBreachProcedure}
              onChange={(v) =>
                onChange({
                  ...answers,
                  existingMeasures: { ...answers.existingMeasures, hasBreachProcedure: v },
                })
              }
            />
            <Toggle
              label="Charte informatique"
              checked={answers.existingMeasures.hasItCharter}
              onChange={(v) =>
                onChange({
                  ...answers,
                  existingMeasures: { ...answers.existingMeasures, hasItCharter: v },
                })
              }
            />
            <Toggle
              label="Charte photos / droit à l'image"
              checked={answers.existingMeasures.hasPhotoCharter}
              onChange={(v) =>
                onChange({
                  ...answers,
                  existingMeasures: { ...answers.existingMeasures, hasPhotoCharter: v },
                })
              }
            />
          </div>
        )}

        {step === 7 && (
          <div className="space-y-3">
            <p className="text-sm font-bold text-slate-800">Synthèse — documents applicables</p>
            <ul className="text-sm space-y-2 max-h-64 overflow-y-auto">
              {requirements.map((r) => {
                const entry = RGPD_CATALOG.find((c) => c.id === r.docId);
                return (
                  <li
                    key={r.docId}
                    className={`rounded-lg px-3 py-2 border ${
                      r.applicable
                        ? "bg-amber-50 border-amber-100 text-amber-950"
                        : "bg-slate-50 border-slate-200 text-slate-500"
                    }`}
                  >
                    <strong>{entry?.title ?? r.docId}</strong> —{" "}
                    {r.applicable ? "Requis" : "Non applicable"} ({r.reason})
                  </li>
                );
              })}
            </ul>
          </div>
        )}

        <div className="flex justify-between mt-6 pt-4 border-t">
          <button
            type="button"
            disabled={step <= 1 || saving}
            onClick={() => void go(step - 1)}
            className="px-4 py-2 text-sm font-bold text-slate-600 disabled:opacity-40"
          >
            Précédent
          </button>
          <button
            type="button"
            disabled={saving}
            onClick={() => void go(step < RGPD_TOTAL_STEPS ? step + 1 : step)}
            className="px-5 py-2 bg-indigo-600 text-white text-sm font-bold rounded-xl disabled:opacity-50"
          >
            {saving ? "Enregistrement…" : step >= RGPD_TOTAL_STEPS ? "Terminer" : "Suivant"}
          </button>
        </div>
      </div>
    </div>
  );
}
