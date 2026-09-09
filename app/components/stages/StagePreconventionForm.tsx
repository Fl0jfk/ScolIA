"use client";

import type {
  StageConvention,
  StageDaySlot,
  StageScheduleMode,
  StageWeekday,
} from "@/app/lib/stage-types";
import type { StageClassPeriod, StagePeriodReminder } from "@/app/lib/stage-periods-config";
import {
  STAGE_WEEKDAY_LABELS,
  STAGE_WEEKDAYS,
  buildPerDaySlotsFromTemplate,
  buildUniformWeekDays,
  defaultDayHoursTemplate,
  formatDaySlotLabel,
} from "@/app/lib/stage-schedule";

const LEVELS = ["6e", "5e", "4e", "3e", "2nde", "1re", "Tle"];

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
  const selectedWeekdays = new Set(
    schedule.days
      .map((d) => d.weekday)
      .filter((w): w is StageWeekday => typeof w === "number" && w >= 1 && w <= 6),
  );

  function updateSchedule(patch: Partial<typeof schedule>) {
    onChange({ ...convention, schedule: { ...schedule, ...patch } });
  }

  function applyHoursToSelectedDays(patch: Partial<StageDaySlot>) {
    const nextDays =
      schedule.mode === "uniform_week"
        ? schedule.days.map((d) => ({ ...d, ...patch, date: undefined }))
        : schedule.days;
    if (schedule.mode === "uniform_week") {
      updateSchedule({ days: nextDays.length ? nextDays : buildUniformWeekDays({ ...hoursTemplate, ...patch }) });
      return;
    }
    updateSchedule({ days: nextDays });
  }

  function updateDay(index: number, patch: Partial<StageDaySlot>) {
    const days = [...(schedule.days || [])];
    days[index] = { ...(days[index] || defaultDayHoursTemplate()), ...patch };
    updateSchedule({ days });
  }

  function toggleWeekday(weekday: StageWeekday, enabled: boolean) {
    const template = { ...hoursTemplate };
    delete template.date;
    let weekdays = [...selectedWeekdays];
    if (enabled) {
      if (!weekdays.includes(weekday)) weekdays.push(weekday);
    } else {
      weekdays = weekdays.filter((w) => w !== weekday);
    }
    weekdays.sort((a, b) => a - b);
    updateSchedule({
      mode: "uniform_week",
      days: buildUniformWeekDays(template, weekdays),
    });
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
    onChange({
      ...convention,
      parent2SignerEmail: value.trim() || undefined,
      student: {
        ...convention.student,
        parent2Email: value.trim() || undefined,
      },
    });
  }

  const parent1Value =
    convention.parentSignerEmail ||
    convention.student.parent1Email ||
    convention.student.parentEmail ||
    "";
  const parent2Value =
    convention.parent2SignerEmail || convention.student.parent2Email || "";

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
        <div className="grid grid-cols-2 gap-2">
          <input
            className="rounded-lg border px-3 py-2 disabled:bg-stone-100 disabled:text-stone-700"
            placeholder="Prénom *"
            value={convention.student.firstName}
            disabled={identityLocked}
            onChange={(e) =>
              onChange({
                ...convention,
                student: { ...convention.student, firstName: e.target.value },
              })
            }
          />
          <input
            className="rounded-lg border px-3 py-2 disabled:bg-stone-100 disabled:text-stone-700"
            placeholder="Nom *"
            value={convention.student.lastName}
            disabled={identityLocked}
            onChange={(e) =>
              onChange({
                ...convention,
                student: { ...convention.student, lastName: e.target.value },
              })
            }
          />
        </div>
        <div className="grid grid-cols-2 gap-2">
          <input
            className="rounded-lg border px-3 py-2 disabled:bg-stone-100 disabled:text-stone-700"
            placeholder="Classe *"
            value={convention.student.className}
            disabled={identityLocked}
            onChange={(e) =>
              onChange({
                ...convention,
                student: { ...convention.student, className: e.target.value },
              })
            }
          />
          <select
            className="rounded-lg border px-3 py-2 disabled:bg-stone-100 disabled:text-stone-700"
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
        </div>
        <input
          className="w-full rounded-lg border px-3 py-2"
          type="email"
          placeholder="E-mail élève (optionnel)"
          value={convention.student.email || ""}
          onChange={(e) =>
            onChange({
              ...convention,
              student: { ...convention.student, email: e.target.value },
            })
          }
        />

        <div className="rounded-xl border border-stone-200 bg-stone-50/80 p-3 space-y-2">
          <p className="text-xs font-bold text-[#1F3D2B]">Responsable(s) légal/aux</p>
          <p className="text-xs text-stone-600 leading-relaxed">
            Un seul responsable suffit pour signer. Le second est optionnel (parents séparés) :
            s&apos;il est renseigné, il recevra aussi l&apos;invitation, mais son absence de
            signature ne bloque pas le dossier.
          </p>
          <input
            className="w-full rounded-lg border px-3 py-2"
            type="email"
            placeholder="E-mail responsable légal qui signe *"
            value={parent1Value}
            onChange={(e) => updateParent1Email(e.target.value)}
          />
          <input
            className="w-full rounded-lg border px-3 py-2"
            type="email"
            placeholder="E-mail 2ᵉ responsable (optionnel)"
            value={parent2Value}
            onChange={(e) => updateParent2Email(e.target.value)}
          />
        </div>

        <p className="rounded-lg border border-stone-200 bg-white px-3 py-2 text-xs text-stone-700">
          Type de stage : <strong>stage en entreprise / observation</strong>
        </p>
      </section>

      <section className="space-y-3">
        <h2 className="text-base font-bold text-[#1F3D2B]">2. Entreprise d&apos;accueil</h2>
        <input
          className="w-full rounded-lg border px-3 py-2"
          placeholder="Raison sociale *"
          value={convention.company.name}
          onChange={(e) =>
            onChange({
              ...convention,
              company: { ...convention.company, name: e.target.value },
            })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          placeholder="Adresse *"
          value={convention.company.address}
          onChange={(e) =>
            onChange({
              ...convention,
              company: { ...convention.company, address: e.target.value },
            })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          placeholder="SIRET (14 chiffres, optionnel)"
          value={convention.company.siret || ""}
          onChange={(e) =>
            onChange({
              ...convention,
              company: { ...convention.company, siret: e.target.value },
            })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          placeholder="Activité de l'entreprise"
          value={convention.company.activity}
          onChange={(e) =>
            onChange({
              ...convention,
              company: { ...convention.company, activity: e.target.value },
            })
          }
        />
        <div className="grid grid-cols-2 gap-2">
          <input
            className="rounded-lg border px-3 py-2"
            placeholder="Tuteur (nom) *"
            value={convention.company.tutorName}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, tutorName: e.target.value },
              })
            }
          />
          <input
            className="rounded-lg border px-3 py-2"
            type="tel"
            placeholder="Tuteur (téléphone)"
            value={convention.company.tutorPhone || ""}
            onChange={(e) =>
              onChange({
                ...convention,
                company: { ...convention.company, tutorPhone: e.target.value },
              })
            }
          />
        </div>
        <input
          className="w-full rounded-lg border px-3 py-2"
          type="email"
          placeholder="Tuteur (e-mail) * — pour envoyer la signature"
          value={convention.company.tutorEmail}
          onChange={(e) =>
            onChange({
              ...convention,
              company: { ...convention.company, tutorEmail: e.target.value },
            })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          type="email"
          placeholder="RH / signataire entreprise — e-mail optionnel"
          value={convention.company.rhEmail || ""}
          onChange={(e) =>
            onChange({
              ...convention,
              company: { ...convention.company, rhEmail: e.target.value },
            })
          }
        />
      </section>

      <section className="space-y-3">
        <h2 className="text-base font-bold text-[#1F3D2B]">3. Période et horaires</h2>
        <div className="grid grid-cols-2 gap-2">
          <label className="text-xs text-stone-600">
            Début
            <input
              type="date"
              className="mt-1 w-full rounded-lg border px-3 py-2"
              value={schedule.periodStart}
              onChange={(e) => updateSchedule({ periodStart: e.target.value })}
            />
          </label>
          <label className="text-xs text-stone-600">
            Fin
            <input
              type="date"
              className="mt-1 w-full rounded-lg border px-3 py-2"
              value={schedule.periodEnd}
              onChange={(e) => updateSchedule({ periodEnd: e.target.value })}
            />
          </label>
        </div>

        <div className="rounded-xl border border-stone-200 bg-white p-4 space-y-3">
          <div>
            <p className="text-xs font-bold text-[#1F3D2B]">Jours de présence</p>
            <p className="mt-1 text-xs text-stone-500">
              Cochez les jours concernés (lundi à samedi). Utile notamment pour la restauration.
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
          {selectedWeekdays.size === 0 && (
            <p className="text-xs text-rose-700">Sélectionnez au moins un jour.</p>
          )}
        </div>

        <div className="space-y-2">
          <p className="text-xs font-bold text-[#1F3D2B]">Horaires (modifiables)</p>
          <p className="text-xs text-stone-500">
            Cliquez sur les heures pour les ajuster — elles s&apos;appliquent à tous les jours
            cochés.
          </p>
          <DayHoursEditor day={hoursTemplate} onPatch={(patch) => applyHoursToSelectedDays(patch)} />
        </div>

        <label className="flex items-center gap-2 text-xs text-stone-600">
          <input
            type="checkbox"
            checked={schedule.mode === "per_day"}
            onChange={(e) => {
              const nextMode = (e.target.checked ? "per_day" : "uniform_week") as StageScheduleMode;
              if (nextMode === "per_day" && schedule.periodStart && schedule.periodEnd) {
                updateSchedule({
                  mode: nextMode,
                  days: buildPerDaySlotsFromTemplate(
                    schedule.periodStart,
                    schedule.periodEnd,
                    hoursTemplate,
                  ),
                });
              } else {
                updateSchedule({
                  mode: "uniform_week",
                  days: buildUniformWeekDays(
                    hoursTemplate,
                    [...selectedWeekdays].sort((a, b) => a - b),
                  ),
                });
              }
            }}
          />
          Détail jour par jour sur la période (avancé)
        </label>

        {schedule.mode === "per_day" && (
          <ul className="max-h-72 space-y-2 overflow-y-auto">
            {schedule.days.map((day, i) => (
              <li key={day.date || i} className="rounded-lg border border-stone-200 p-3">
                <p className="mb-2 text-xs font-bold text-stone-700">{formatDaySlotLabel(day)}</p>
                <DayHoursEditor day={day} onPatch={(patch) => updateDay(i, patch)} />
              </li>
            ))}
          </ul>
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
