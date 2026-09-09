"use client";

import type { ReactNode } from "react";
import type {
  StageConvention,
  StageDaySlot,
  StageScheduleMode,
  StageWeekday,
} from "@/app/lib/stage-types";
import type { StageClassPeriod, StagePeriodReminder } from "@/app/lib/stage-periods-config";
import {
  STAGE_DEFAULT_WEEKDAYS,
  STAGE_WEEKDAY_LABELS,
  STAGE_WEEKDAYS,
  buildUniformWeekDays,
  defaultDayHoursTemplate,
  formatDaySlotLabel,
  rebuildPerDaySlots,
} from "@/app/lib/stage-schedule";

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

function TimeField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <label className="block text-xs font-semibold text-[#1F3D2B]">
      {label}
      <input
        type="time"
        className="mt-1 w-full rounded-lg border-2 border-[#2F6B4A]/40 bg-white px-3 py-2.5 text-sm font-semibold text-[#1F3D2B] shadow-sm focus:border-[#2F6B4A] focus:outline-none focus:ring-2 focus:ring-[#2F6B4A]/30"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  );
}

function DayHoursEditor({
  day,
  onPatch,
}: {
  day: StageDaySlot;
  onPatch: (patch: Partial<StageDaySlot>) => void;
}) {
  return (
    <div className="space-y-3 rounded-xl border border-[#2F6B4A]/25 bg-[#f3faf6] p-3">
      <label className="flex items-center gap-2 text-xs font-semibold text-[#1F3D2B]">
        <input
          type="checkbox"
          checked={day.hasLunchBreak !== false}
          onChange={(e) => onPatch({ hasLunchBreak: e.target.checked })}
        />
        Pause le midi
      </label>
      {day.hasLunchBreak !== false ? (
        <div className="grid grid-cols-2 gap-3">
          <TimeField
            label="Matin — début"
            value={day.morningStart || ""}
            onChange={(v) => onPatch({ morningStart: v })}
          />
          <TimeField
            label="Matin — fin"
            value={day.morningEnd || ""}
            onChange={(v) => onPatch({ morningEnd: v })}
          />
          <TimeField
            label="Après-midi — début"
            value={day.afternoonStart || ""}
            onChange={(v) => onPatch({ afternoonStart: v })}
          />
          <TimeField
            label="Après-midi — fin"
            value={day.afternoonEnd || ""}
            onChange={(v) => onPatch({ afternoonEnd: v })}
          />
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3">
          <TimeField
            label="Journée — début"
            value={day.fullDayStart || day.morningStart || ""}
            onChange={(v) => onPatch({ fullDayStart: v, morningStart: v })}
          />
          <TimeField
            label="Journée — fin"
            value={day.fullDayEnd || day.morningEnd || ""}
            onChange={(v) => onPatch({ fullDayEnd: v, morningEnd: v })}
          />
        </div>
      )}
    </div>
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
}) {
  const schedule = convention.schedule;
  const hoursTemplate = schedule.days[0] || defaultDayHoursTemplate();

  const selectedWeekdaysList: StageWeekday[] = Array.isArray(schedule.presenceWeekdays)
    ? [...schedule.presenceWeekdays].filter((w) => w >= 1 && w <= 6).sort((a, b) => a - b)
    : schedule.mode === "uniform_week"
      ? (() => {
          const fromDays = schedule.days
            .map((d) => d.weekday)
            .filter((w): w is StageWeekday => typeof w === "number" && w >= 1 && w <= 6);
          return fromDays.length > 0
            ? [...new Set(fromDays)].sort((a, b) => a - b)
            : [...STAGE_DEFAULT_WEEKDAYS];
        })()
      : [...STAGE_DEFAULT_WEEKDAYS];
  const selectedWeekdays = new Set(selectedWeekdaysList);

  function updateSchedule(patch: Partial<typeof schedule>) {
    onChange({ ...convention, schedule: { ...schedule, ...patch } });
  }

  function applyUniformHours(patch: Partial<StageDaySlot>) {
    const template = { ...hoursTemplate, ...patch };
    delete template.date;
    delete template.weekday;
    updateSchedule({
      mode: "uniform_week",
      presenceWeekdays: selectedWeekdaysList,
      days:
        selectedWeekdaysList.length === 0
          ? []
          : buildUniformWeekDays(template, selectedWeekdaysList),
    });
  }

  function updateDay(index: number, patch: Partial<StageDaySlot>) {
    const days = [...(schedule.days || [])];
    days[index] = { ...(days[index] || defaultDayHoursTemplate()), ...patch };
    updateSchedule({ days });
  }

  function setPresenceWeekdays(weekdays: StageWeekday[]) {
    const sorted = [...new Set(weekdays)].filter((w) => w >= 1 && w <= 6).sort((a, b) => a - b);
    const template = { ...hoursTemplate };
    delete template.date;
    delete template.weekday;
    if (schedule.mode === "per_day") {
      updateSchedule({
        presenceWeekdays: sorted,
        days:
          sorted.length === 0
            ? []
            : rebuildPerDaySlots({
                periodStart: schedule.periodStart,
                periodEnd: schedule.periodEnd,
                weekdays: sorted,
                existing: schedule.days,
                template,
              }),
      });
      return;
    }
    updateSchedule({
      mode: "uniform_week",
      presenceWeekdays: sorted,
      days: sorted.length === 0 ? [] : buildUniformWeekDays(template, sorted),
    });
  }

  function toggleWeekday(weekday: StageWeekday, enabled: boolean) {
    let weekdays = [...selectedWeekdaysList];
    if (enabled) {
      if (!weekdays.includes(weekday)) weekdays.push(weekday);
    } else {
      weekdays = weekdays.filter((w) => w !== weekday);
    }
    setPresenceWeekdays(weekdays);
  }

  function setScheduleMode(nextMode: StageScheduleMode) {
    const template = { ...hoursTemplate };
    delete template.date;
    delete template.weekday;
    if (nextMode === "per_day") {
      updateSchedule({
        mode: "per_day",
        presenceWeekdays: selectedWeekdaysList,
        days:
          selectedWeekdaysList.length === 0
            ? []
            : rebuildPerDaySlots({
                periodStart: schedule.periodStart,
                periodEnd: schedule.periodEnd,
                weekdays: selectedWeekdaysList,
                existing: schedule.days,
                template,
              }),
      });
      return;
    }
    updateSchedule({
      mode: "uniform_week",
      presenceWeekdays: selectedWeekdaysList,
      days:
        selectedWeekdaysList.length === 0
          ? []
          : buildUniformWeekDays(template, selectedWeekdaysList),
    });
  }

  function setPeriodBounds(patch: { periodStart?: string; periodEnd?: string }) {
    const periodStart = patch.periodStart ?? schedule.periodStart;
    const periodEnd = patch.periodEnd ?? schedule.periodEnd;
    if (schedule.mode === "per_day") {
      const template = { ...hoursTemplate };
      delete template.date;
      delete template.weekday;
      updateSchedule({
        periodStart,
        periodEnd,
        presenceWeekdays: selectedWeekdaysList,
        days: rebuildPerDaySlots({
          periodStart,
          periodEnd,
          weekdays: selectedWeekdaysList,
          existing: schedule.days,
          template,
        }),
      });
      return;
    }
    updateSchedule({ periodStart, periodEnd, presenceWeekdays: selectedWeekdaysList });
  }

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
        <Field label="E-mail élève" hint="Optionnel">
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
          <Field label="E-mail du 2ᵉ responsable" hint="Optionnel — laissez vide s'il n'y a qu'un responsable">
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
        <Field label="Adresse" required>
          <input
            className={fieldInputClass}
            value={convention.company.address}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, address: e.target.value },
              })
            }
          />
        </Field>
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
        <Field label="RH / signataire entreprise — e-mail" hint="Optionnel si signataire">
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
        <div className="grid grid-cols-2 gap-3">
          <Field label="Date de début" required>
            <input
              type="date"
              className={fieldInputClass}
              value={schedule.periodStart}
              onChange={(e) => setPeriodBounds({ periodStart: e.target.value })}
            />
          </Field>
          <Field label="Date de fin" required>
            <input
              type="date"
              className={fieldInputClass}
              value={schedule.periodEnd}
              onChange={(e) => setPeriodBounds({ periodEnd: e.target.value })}
            />
          </Field>
        </div>

        <div className="space-y-2">
          <p className="text-xs font-bold text-[#1F3D2B]">Comment sont vos horaires ?</p>
          <div className="grid gap-2 sm:grid-cols-2">
            <button
              type="button"
              onClick={() => setScheduleMode("uniform_week")}
              className={`rounded-xl border-2 px-4 py-3 text-left transition ${
                schedule.mode === "uniform_week"
                  ? "border-[#2F6B4A] bg-[#e8f5ee] shadow-sm"
                  : "border-stone-200 bg-white hover:border-stone-300"
              }`}
            >
              <span className="block text-sm font-bold text-[#1F3D2B]">
                Mêmes horaires tous les jours
              </span>
              <span className="mt-1 block text-xs text-stone-600 leading-snug">
                Vous configurez une seule fois — appliqué à chaque jour coché.
              </span>
            </button>
            <button
              type="button"
              onClick={() => setScheduleMode("per_day")}
              className={`rounded-xl border-2 px-4 py-3 text-left transition ${
                schedule.mode === "per_day"
                  ? "border-[#2F6B4A] bg-[#e8f5ee] shadow-sm"
                  : "border-stone-200 bg-white hover:border-stone-300"
              }`}
            >
              <span className="block text-sm font-bold text-[#1F3D2B]">
                Horaires différents chaque jour
              </span>
              <span className="mt-1 block text-xs text-stone-600 leading-snug">
                Chaque date de la période se règle à part (semaines 1 et 2 peuvent différer).
              </span>
            </button>
          </div>
        </div>

        <div className="rounded-xl border border-stone-200 bg-white p-4 space-y-3">
          <div>
            <p className="text-xs font-bold text-[#1F3D2B]">Jours de présence dans la semaine</p>
            <p className="mt-1 text-xs text-stone-500">
              Cochez les jours concernés (lundi à samedi) — utile notamment pour la restauration.
            </p>
          </div>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
            {STAGE_WEEKDAYS.map((weekday) => {
              const checked = selectedWeekdays.has(weekday);
              return (
                <label
                  key={weekday}
                  className={`flex cursor-pointer items-center gap-2 rounded-lg border px-3 py-2 text-sm font-semibold transition ${
                    checked
                      ? "border-[#2F6B4A] bg-[#e8f5ee] text-[#1F3D2B]"
                      : "border-stone-200 bg-stone-50 text-stone-500"
                  }`}
                >
                  <input
                    type="checkbox"
                    className="accent-[#2F6B4A]"
                    checked={checked}
                    onChange={(e) => toggleWeekday(weekday, e.target.checked)}
                  />
                  {STAGE_WEEKDAY_LABELS[weekday]}
                </label>
              );
            })}
          </div>
          {selectedWeekdaysList.length === 0 && (
            <p className="text-xs text-rose-700">Sélectionnez au moins un jour.</p>
          )}
        </div>

        {schedule.mode === "uniform_week" ? (
          <div className="space-y-2">
            <p className="text-xs font-bold text-[#1F3D2B]">Horaires (une seule configuration)</p>
            <p className="text-xs text-stone-500">
              Ces horaires s&apos;appliquent à tous les jours cochés, chaque semaine de la période.
            </p>
            <DayHoursEditor day={hoursTemplate} onPatch={(patch) => applyUniformHours(patch)} />
          </div>
        ) : (
          <div className="space-y-3">
            <div>
              <p className="text-xs font-bold text-[#1F3D2B]">Horaires jour par jour</p>
              <p className="mt-1 text-xs text-stone-500 leading-relaxed">
                Chaque date de votre période apparaît ci-dessous. Ajustez librement : la 1ʳᵉ semaine
                peut avoir des horaires différents de la 2ᵉ.
              </p>
            </div>
            {schedule.days.length === 0 ? (
              <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-800">
                Aucun jour dans la période avec les cases cochées. Élargissez les dates ou cochez
                d&apos;autres jours.
              </p>
            ) : (
              <ul className="max-h-96 space-y-3 overflow-y-auto pr-1">
                {schedule.days.map((day, i) => (
                  <li key={day.date || i} className="rounded-xl border border-stone-200 bg-white p-3">
                    <p className="mb-2 text-sm font-bold text-[#1F3D2B]">
                      {day.date
                        ? new Date(`${day.date}T12:00:00`).toLocaleDateString("fr-FR", {
                            weekday: "long",
                            day: "numeric",
                            month: "long",
                          })
                        : formatDaySlotLabel(day)}
                    </p>
                    <DayHoursEditor day={day} onPatch={(patch) => updateDay(i, patch)} />
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </section>

      {convention.teacherReferent.name && (
        <section className="rounded-xl border border-blue-100 bg-blue-50/60 p-4 text-xs text-blue-900">
          <p className="font-bold">Professeur principal / référent</p>
          <p className="mt-1">
            {convention.teacherReferent.name} — {convention.teacherReferent.email}
          </p>
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
          Envoyer à l&apos;administratif
        </button>
      </div>
    </div>
  );
}
