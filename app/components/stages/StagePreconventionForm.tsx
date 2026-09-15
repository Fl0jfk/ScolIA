"use client";

import type { ReactNode } from "react";
import type { StageConvention } from "@/app/lib/stage-types";
import type { StageClassPeriod, StagePeriodReminder } from "@/app/lib/stage-periods-config";
import type {
  StageBlockedPeriod,
  StageCycleConstraints,
} from "@/app/lib/stage-constraints";
import StageScheduleEditor from "@/app/components/stages/StageScheduleEditor";

const LEVELS = ["6e", "5e", "4e", "3e", "2nde", "1re", "Tle"];

const fieldInputClass =
  "mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm text-stone-900 disabled:bg-stone-100 disabled:text-stone-700";

function Field({
  label,
  required,
  hint,
  children,
}: {
  label: string;
  required?: boolean;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="block">
      <span className="text-xs font-semibold text-stone-600">
        {label}
        {required ? " *" : ""}
      </span>
      {children}
      {hint ? <span className="mt-1 block text-[11px] font-normal text-stone-500">{hint}</span> : null}
    </label>
  );
}

export default function StagePreconventionForm({
  convention,
  onChange,
  onSave,
  onSubmit,
  busy,
  identityLocked = false,
  reminders = [],
  officialPeriods = [],
  showAdminHint = false,
  submitLabel = "Envoyer à l'administratif",
  scheduleConstraints,
  cycleLabel,
}: {
  convention: StageConvention;
  onChange: (next: StageConvention) => void;
  onSave: () => void;
  onSubmit: () => void;
  busy: boolean;
  /** Identité vérifiée via nom + prénom + date de naissance — champs élève non modifiables. */
  identityLocked?: boolean;
  reminders?: StagePeriodReminder[];
  officialPeriods?: StageClassPeriod[];
  showAdminHint?: boolean;
  /** Libellé du bouton d'envoi / validation (ex. page publique élève). */
  submitLabel?: string;
  scheduleConstraints?: {
    rules: StageCycleConstraints;
    blockedPeriods: StageBlockedPeriod[];
  } | null;
  cycleLabel?: string;
}) {
  function updateParent1Email(value: string) {
    onChange({
      ...convention,
      parentSignerEmail: value,
      student: {
        ...convention.student,
        parent1Email: value,
        parentEmail: value,
      },
    });
  }

  function updateParent2Email(value: string) {
    const cleared = value.trim();
    onChange({
      ...convention,
      // Chaîne vide explicite pour que le serveur n'ait pas à retomber sur l'ancien e-mail.
      parent2SignerEmail: cleared,
      student: {
        ...convention.student,
        parent2Email: cleared,
      },
    });
  }

  const parent1Value =
    convention.parentSignerEmail ||
    convention.student.parent1Email ||
    convention.student.parentEmail ||
    "";
  const parent2Value =
    typeof convention.parent2SignerEmail === "string"
      ? convention.parent2SignerEmail
      : typeof convention.student.parent2Email === "string"
        ? convention.student.parent2Email
        : "";

  return (
    <div className="space-y-8 text-sm">
      {(reminders.length > 0 || officialPeriods.length > 0) && (
        <section className="rounded-xl border border-amber-200 bg-amber-50/80 p-4 space-y-3">
          <h2 className="text-sm font-bold text-amber-900">
            Rappels — dates habituelles pour votre classe
          </h2>
          <p className="text-xs text-amber-900/90 leading-relaxed">
            Indicatif : vous pouvez demander un stage hors de ces dates ; l&apos;établissement
            validera ensuite.
          </p>
          {officialPeriods.map((p) => (
            <div key={p.id} className="text-xs text-amber-900">
              <p className="font-semibold">{p.label}</p>
              <p>
                Du {new Date(p.periodStart).toLocaleDateString("fr-FR")} au{" "}
                {new Date(p.periodEnd).toLocaleDateString("fr-FR")}
              </p>
            </div>
          ))}
          {reminders.map((r) => (
            <div
              key={r.id}
              className="text-xs text-amber-900 border-t border-amber-200/60 pt-2 first:border-0 first:pt-0"
            >
              <p className="font-semibold">{r.label}</p>
              <p className="mt-0.5 whitespace-pre-wrap">{r.message}</p>
              {r.periodStart && r.periodEnd && (
                <p className="mt-1 text-amber-800">
                  Période indicative : {new Date(r.periodStart).toLocaleDateString("fr-FR")} →{" "}
                  {new Date(r.periodEnd).toLocaleDateString("fr-FR")}
                </p>
              )}
            </div>
          ))}
        </section>
      )}

      {showAdminHint && (
        <p className="rounded-lg border border-blue-200 bg-blue-50 px-3 py-2 text-xs text-blue-900">
          Modification administrative — vous pouvez corriger les champs avant validation ou renvoyer
          le dossier aux responsables légaux pour correction.
        </p>
      )}

      <section className="space-y-3">
        <h2 className="text-base font-bold text-[#1F3D2B]">1. Identité élève</h2>
        {identityLocked && (
          <p className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-900">
            Identité confirmée par l&apos;établissement (nom, prénom et date de naissance). Les
            champs ci-dessous ne sont pas modifiables.
          </p>
        )}
        <div className="grid grid-cols-2 gap-3">
          <Field label="Prénom" required>
            <input
              className={fieldInputClass}
              value={convention.student.firstName}
              disabled={identityLocked}
              onChange={(e) =>
                onChange({
                  ...convention,
                  student: { ...convention.student, firstName: e.target.value },
                })
              }
            />
          </Field>
          <Field label="Nom" required>
            <input
              className={fieldInputClass}
              value={convention.student.lastName}
              disabled={identityLocked}
              onChange={(e) =>
                onChange({
                  ...convention,
                  student: { ...convention.student, lastName: e.target.value },
                })
              }
            />
          </Field>
        </div>
        <div className="grid grid-cols-2 gap-3">
          <Field label="Classe" required>
            <input
              className={fieldInputClass}
              value={convention.student.className}
              disabled={identityLocked}
              onChange={(e) =>
                onChange({
                  ...convention,
                  student: { ...convention.student, className: e.target.value },
                })
              }
            />
          </Field>
          <Field label="Niveau" required>
            <select
              className={fieldInputClass}
              value={convention.student.level}
              disabled={identityLocked}
              onChange={(e) =>
                onChange({
                  ...convention,
                  student: { ...convention.student, level: e.target.value },
                })
              }
            >
              {LEVELS.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </select>
          </Field>
        </div>
        <Field label="E-mail élève" hint="Optionnel — recommandé pour le suivi">
          <input
            className={fieldInputClass}
            type="email"
            value={convention.student.email || ""}
            onChange={(e) =>
              onChange({
                ...convention,
                student: { ...convention.student, email: e.target.value },
              })
            }
          />
        </Field>
        <p className="rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-[11px] leading-relaxed text-sky-950">
          Pourquoi votre e-mail ? Pour le suivi du stage : si l&apos;établissement demande des
          modifications ou refuse la préconvention, vous recevrez la notification directement.
        </p>

        <div className="rounded-xl border border-stone-200 bg-stone-50/80 p-3 space-y-3">
          <p className="text-xs font-bold text-[#1F3D2B]">Responsable(s) légal/aux</p>
          <p className="text-xs text-stone-600 leading-relaxed">
            Un seul responsable suffit pour signer. Le second est optionnel (parents séparés) :
            s&apos;il est renseigné, il recevra aussi l&apos;invitation, mais son absence de
            signature ne bloque pas le dossier.
          </p>
          <Field label="E-mail du responsable qui signe" required>
            <input
              className={fieldInputClass}
              type="email"
              value={parent1Value}
              onChange={(e) => updateParent1Email(e.target.value)}
            />
          </Field>
          <Field
            label="E-mail du 2ᵉ responsable"
            hint="Optionnel — laissez vide s'il n'y a qu'un responsable"
          >
            <input
              className={fieldInputClass}
              type="email"
              value={parent2Value}
              onChange={(e) => updateParent2Email(e.target.value)}
            />
          </Field>
          {parent2Value.trim() ? (
            <button
              type="button"
              className="text-xs font-semibold text-stone-600 underline"
              onClick={() => updateParent2Email("")}
            >
              Retirer le 2ᵉ responsable
            </button>
          ) : null}
        </div>

        <p className="rounded-lg border border-stone-200 bg-white px-3 py-2 text-xs text-stone-700">
          Type de stage : <strong>stage en entreprise / observation</strong>
        </p>
      </section>

      <section className="space-y-3">
        <h2 className="text-base font-bold text-[#1F3D2B]">2. Entreprise d&apos;accueil</h2>
        <Field label="Raison sociale" required>
          <input
            className={fieldInputClass}
            value={convention.company.name}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, name: e.target.value },
              })
            }
          />
        </Field>
        <Field label="Adresse (rue et numéro)" required>
          <input
            className={fieldInputClass}
            value={convention.company.address}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, address: e.target.value },
              })
            }
            autoComplete="street-address"
            placeholder="ex. 12 rue de la République"
          />
        </Field>
        <div className="grid gap-3 sm:grid-cols-[8rem_1fr]">
          <Field label="Code postal" required>
            <input
              className={fieldInputClass}
              value={convention.company.postalCode || ""}
              onChange={(e) =>
                onChange({
                  ...convention,
                  company: {
                    ...convention.company,
                    postalCode: e.target.value.replace(/\D/g, "").slice(0, 5),
                  },
                })
              }
              inputMode="numeric"
              autoComplete="postal-code"
              placeholder="76000"
              maxLength={5}
            />
          </Field>
          <Field label="Ville" required>
            <input
              className={fieldInputClass}
              value={convention.company.city || ""}
              onChange={(e) =>
                onChange({
                  ...convention,
                  company: { ...convention.company, city: e.target.value },
                })
              }
              autoComplete="address-level2"
              placeholder="ex. Rouen"
            />
          </Field>
        </div>
        <Field label="SIRET" hint="14 chiffres — optionnel">
          <input
            className={fieldInputClass}
            value={convention.company.siret || ""}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, siret: e.target.value },
              })
            }
          />
        </Field>
        <Field label="Activité de l'entreprise">
          <input
            className={fieldInputClass}
            value={convention.company.activity}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, activity: e.target.value },
              })
            }
          />
        </Field>
        <div className="grid grid-cols-2 gap-3">
          <Field label="Tuteur — nom" required>
            <input
              className={fieldInputClass}
              value={convention.company.tutorName}
              onChange={(e) =>
                onChange({
                  ...convention,
                  company: { ...convention.company, tutorName: e.target.value },
                })
              }
            />
          </Field>
          <Field label="Tuteur — téléphone">
            <input
              className={fieldInputClass}
              type="tel"
              value={convention.company.tutorPhone || ""}
              onChange={(e) =>
                onChange({
                  ...convention,
                  company: { ...convention.company, tutorPhone: e.target.value },
                })
              }
            />
          </Field>
        </div>
        <Field
          label="Tuteur — e-mail"
          required
          hint="Obligatoire pour envoyer la convention à signer"
        >
          <input
            className={fieldInputClass}
            type="email"
            value={convention.company.tutorEmail}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, tutorEmail: e.target.value },
              })
            }
          />
        </Field>
        <Field
          label="RH / signataire entreprise — e-mail"
          hint="Optionnel — si renseigné, cette personne reçoit aussi un lien pour signer (en plus du tuteur)."
        >
          <input
            className={fieldInputClass}
            type="email"
            value={convention.company.rhEmail || ""}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, rhEmail: e.target.value },
              })
            }
          />
        </Field>
      </section>

      <section className="space-y-4">
        <h2 className="text-base font-bold text-[#1F3D2B]">3. Période et horaires</h2>
        <StageScheduleEditor
          value={convention.schedule}
          onChange={(schedule) => onChange({ ...convention, schedule })}
          title="Dates, jours et créneaux"
          constraints={scheduleConstraints}
          dateNaissance={convention.student.dateNaissance}
          cycleLabel={cycleLabel}
        />
      </section>

      {convention.teacherReferent.name && (
        <section className="rounded-xl border border-blue-100 bg-blue-50/60 p-4 text-xs text-blue-900">
          <p className="font-bold">Professeur principal / référent</p>
          <p className="mt-1">{convention.teacherReferent.name}</p>
        </section>
      )}

      <div className="flex flex-wrap gap-2 pt-2">
        <button
          type="button"
          disabled={busy}
          onClick={onSave}
          className="rounded-lg border border-stone-300 px-4 py-2 font-semibold"
        >
          Enregistrer le brouillon
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={onSubmit}
          className="rounded-lg bg-[#2F6B4A] px-4 py-2 font-semibold text-white"
        >
          {submitLabel}
        </button>
      </div>
    </div>
  );
}
