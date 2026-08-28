"use client";

import type { StageConvention, StageDaySlot, StageScheduleMode } from "@/app/lib/stage-types";
import type { StageClassPeriod, StagePeriodReminder } from "@/app/lib/stage-periods-config";
import { STAGE_OFFER_KIND_LABELS } from "@/app/lib/stage-types";
import { buildPerDaySlotsFromTemplate, formatDaySlotLabel } from "@/app/lib/stage-schedule";

const LEVELS = ["6e", "5e", "4e", "3e", "2nde", "1re", "Tle"];

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
  /** Identité vérifiée via INE + date de naissance — champs élève non modifiables. */
  identityLocked?: boolean;
  reminders?: StagePeriodReminder[];
  officialPeriods?: StageClassPeriod[];
  showAdminHint?: boolean;
}) {
  const schedule = convention.schedule;

  function updateSchedule(patch: Partial<typeof schedule>) {
    onChange({ ...convention, schedule: { ...schedule, ...patch } });
  }

  function updateDay(index: number, patch: Partial<StageDaySlot>) {
    const days = [...(schedule.days || [])];
    days[index] = { ...(days[index] || { hasLunchBreak: true }), ...patch };
    updateSchedule({ days });
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
      parent2SignerEmail: value,
      student: { ...convention.student, parent2Email: value },
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
          <h2 className="text-sm font-bold text-amber-900">Rappels — dates de stage</h2>
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
            <div key={r.id} className="text-xs text-amber-900 border-t border-amber-200/60 pt-2 first:border-0 first:pt-0">
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
            Identité confirmée par l&apos;établissement (INE + date de naissance). Les champs ci-dessous
            ne sont pas modifiables.
          </p>
        )}
        <div className="grid grid-cols-2 gap-2">
          <input
            className="rounded-lg border px-3 py-2 disabled:bg-stone-100 disabled:text-stone-700"
            placeholder="Prénom *"
            value={convention.student.firstName}
            disabled={identityLocked}
            onChange={(e) =>
              onChange({ ...convention, student: { ...convention.student, firstName: e.target.value } })
            }
          />
          <input
            className="rounded-lg border px-3 py-2 disabled:bg-stone-100 disabled:text-stone-700"
            placeholder="Nom *"
            value={convention.student.lastName}
            disabled={identityLocked}
            onChange={(e) =>
              onChange({ ...convention, student: { ...convention.student, lastName: e.target.value } })
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
              onChange({ ...convention, student: { ...convention.student, className: e.target.value } })
            }
          />
          <select
            className="rounded-lg border px-3 py-2 disabled:bg-stone-100 disabled:text-stone-700"
            value={convention.student.level}
            disabled={identityLocked}
            onChange={(e) =>
              onChange({ ...convention, student: { ...convention.student, level: e.target.value } })
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
            onChange({ ...convention, student: { ...convention.student, email: e.target.value } })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          type="email"
          placeholder="E-mail responsable légal 1 *"
          value={parent1Value}
          onChange={(e) => updateParent1Email(e.target.value)}
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          type="email"
          placeholder="E-mail responsable légal 2 *"
          value={parent2Value}
          onChange={(e) => updateParent2Email(e.target.value)}
        />
        <p className="text-xs text-stone-500">
          Les deux responsables légaux recevront les e-mails de correction et de signature.
        </p>
        <select
          className="w-full rounded-lg border px-3 py-2"
          value={convention.internshipKind}
          onChange={(e) =>
            onChange({
              ...convention,
              internshipKind: e.target.value as StageConvention["internshipKind"],
            })
          }
        >
          {Object.entries(STAGE_OFFER_KIND_LABELS).map(([k, label]) => (
            <option key={k} value={k}>
              {label}
            </option>
          ))}
        </select>
      </section>

      <section className="space-y-3">
        <h2 className="text-base font-bold text-[#1F3D2B]">2. Entreprise d&apos;accueil</h2>
        <input
          className="w-full rounded-lg border px-3 py-2"
          placeholder="Raison sociale *"
          value={convention.company.name}
          onChange={(e) =>
            onChange({ ...convention, company: { ...convention.company, name: e.target.value } })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          placeholder="Adresse *"
          value={convention.company.address}
          onChange={(e) =>
            onChange({ ...convention, company: { ...convention.company, address: e.target.value } })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          placeholder="SIRET (14 chiffres) *"
          value={convention.company.siret || ""}
          onChange={(e) =>
            onChange({ ...convention, company: { ...convention.company, siret: e.target.value } })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          placeholder="Activité de l'entreprise"
          value={convention.company.activity}
          onChange={(e) =>
            onChange({ ...convention, company: { ...convention.company, activity: e.target.value } })
          }
        />
        <div className="grid grid-cols-2 gap-2">
          <input
            className="rounded-lg border px-3 py-2"
            placeholder="Tuteur (nom) *"
            value={convention.company.tutorName}
            onChange={(e) =>
              onChange({ ...convention, company: { ...convention.company, tutorName: e.target.value } })
            }
          />
          <input
            className="rounded-lg border px-3 py-2"
            type="tel"
            placeholder="Tuteur (téléphone)"
            value={convention.company.tutorPhone || ""}
            onChange={(e) =>
              onChange({ ...convention, company: { ...convention.company, tutorPhone: e.target.value } })
            }
          />
        </div>
        <input
          className="w-full rounded-lg border px-3 py-2"
          type="email"
          placeholder="Tuteur (e-mail) *"
          value={convention.company.tutorEmail}
          onChange={(e) =>
            onChange({ ...convention, company: { ...convention.company, tutorEmail: e.target.value } })
          }
        />
        <input
          className="w-full rounded-lg border px-3 py-2"
          type="email"
          placeholder="RH entreprise (e-mail, optionnel)"
          value={convention.company.rhEmail || ""}
          onChange={(e) =>
            onChange({ ...convention, company: { ...convention.company, rhEmail: e.target.value } })
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
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={schedule.mode === "uniform_week"}
            onChange={(e) =>
              updateSchedule({
                mode: (e.target.checked ? "uniform_week" : "per_day") as StageScheduleMode,
              })
            }
          />
          Mêmes horaires tous les jours de la semaine (lun–ven)
        </label>

        {schedule.mode === "per_day" && schedule.periodStart && schedule.periodEnd && (
          <button
            type="button"
            className="text-xs font-semibold text-[#2F6B4A] underline"
            onClick={() => {
              const template = schedule.days[0] || {
                hasLunchBreak: true,
                morningStart: "08:00",
                morningEnd: "12:00",
                afternoonStart: "13:00",
                afternoonEnd: "17:00",
              };
              updateSchedule({
                days: buildPerDaySlotsFromTemplate(
                  schedule.periodStart,
                  schedule.periodEnd,
                  template,
                ),
              });
            }}
          >
            Générer un créneau par jour ouvré
          </button>
        )}

        {schedule.mode === "uniform_week" ? (
          <div className="rounded-xl border border-stone-200 bg-stone-50 p-4 space-y-2">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={schedule.days[0]?.hasLunchBreak !== false}
                onChange={(e) => updateDay(0, { hasLunchBreak: e.target.checked })}
              />
              Pause le midi
            </label>
            {schedule.days[0]?.hasLunchBreak !== false ? (
              <div className="grid grid-cols-2 gap-2">
                <input type="time" value={schedule.days[0]?.morningStart || ""} onChange={(e) => updateDay(0, { morningStart: e.target.value })} />
                <input type="time" value={schedule.days[0]?.morningEnd || ""} onChange={(e) => updateDay(0, { morningEnd: e.target.value })} />
                <input type="time" value={schedule.days[0]?.afternoonStart || ""} onChange={(e) => updateDay(0, { afternoonStart: e.target.value })} />
                <input type="time" value={schedule.days[0]?.afternoonEnd || ""} onChange={(e) => updateDay(0, { afternoonEnd: e.target.value })} />
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2">
                <input type="time" value={schedule.days[0]?.fullDayStart || ""} onChange={(e) => updateDay(0, { fullDayStart: e.target.value })} />
                <input type="time" value={schedule.days[0]?.fullDayEnd || ""} onChange={(e) => updateDay(0, { fullDayEnd: e.target.value })} />
              </div>
            )}
          </div>
        ) : (
          <ul className="space-y-2 max-h-64 overflow-y-auto">
            {schedule.days.map((day, i) => (
              <li key={day.date || i} className="rounded-lg border border-stone-200 p-3">
                <p className="text-xs font-bold text-stone-700 mb-2">{formatDaySlotLabel(day)}</p>
                <label className="flex items-center gap-2 text-xs mb-2">
                  <input
                    type="checkbox"
                    checked={day.hasLunchBreak !== false}
                    onChange={(e) => updateDay(i, { hasLunchBreak: e.target.checked })}
                  />
                  Pause midi
                </label>
                {day.hasLunchBreak !== false ? (
                  <div className="grid grid-cols-2 gap-1">
                    <input type="time" className="rounded border px-2 py-1" value={day.morningStart || ""} onChange={(e) => updateDay(i, { morningStart: e.target.value })} />
                    <input type="time" className="rounded border px-2 py-1" value={day.morningEnd || ""} onChange={(e) => updateDay(i, { morningEnd: e.target.value })} />
                    <input type="time" className="rounded border px-2 py-1" value={day.afternoonStart || ""} onChange={(e) => updateDay(i, { afternoonStart: e.target.value })} />
                    <input type="time" className="rounded border px-2 py-1" value={day.afternoonEnd || ""} onChange={(e) => updateDay(i, { afternoonEnd: e.target.value })} />
                  </div>
                ) : (
                  <div className="grid grid-cols-2 gap-1">
                    <input type="time" className="rounded border px-2 py-1" value={day.fullDayStart || ""} onChange={(e) => updateDay(i, { fullDayStart: e.target.value })} />
                    <input type="time" className="rounded border px-2 py-1" value={day.fullDayEnd || ""} onChange={(e) => updateDay(i, { fullDayEnd: e.target.value })} />
                  </div>
                )}
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
