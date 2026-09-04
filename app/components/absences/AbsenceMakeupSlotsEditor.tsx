"use client";

import {
  emptyMakeupSlotDraft,
  type MakeupSlotDraft,
} from "@/app/lib/absence-hours-treatment";

type Props = {
  slots: MakeupSlotDraft[];
  onChange: (slots: MakeupSlotDraft[]) => void;
  idPrefix?: string;
};

export default function AbsenceMakeupSlotsEditor({ slots, onChange, idPrefix = "makeup" }: Props) {
  const rows = slots.length > 0 ? slots : [emptyMakeupSlotDraft()];

  const updateRow = (index: number, patch: Partial<MakeupSlotDraft>) => {
    const next = rows.map((row, i) => (i === index ? { ...row, ...patch } : row));
    onChange(next);
  };

  const removeRow = (index: number) => {
    if (rows.length <= 1) {
      onChange([emptyMakeupSlotDraft()]);
      return;
    }
    onChange(rows.filter((_, i) => i !== index));
  };

  return (
    <div className="space-y-2">
      {rows.map((slot, index) => (
        <div
          key={`${idPrefix}-${index}`}
          className="grid grid-cols-1 sm:grid-cols-[1.2fr_0.8fr_0.8fr_auto] gap-2 items-end"
        >
          <div>
            <label
              htmlFor={`${idPrefix}-date-${index}`}
              className="text-[10px] font-black uppercase tracking-wider text-slate-500 block mb-1"
            >
              Jour
            </label>
            <input
              id={`${idPrefix}-date-${index}`}
              type="date"
              value={slot.date}
              onChange={(e) => updateRow(index, { date: e.target.value })}
              className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm bg-white"
            />
          </div>
          <div>
            <label
              htmlFor={`${idPrefix}-start-${index}`}
              className="text-[10px] font-black uppercase tracking-wider text-slate-500 block mb-1"
            >
              De
            </label>
            <input
              id={`${idPrefix}-start-${index}`}
              type="time"
              value={slot.startTime}
              onChange={(e) => updateRow(index, { startTime: e.target.value })}
              className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm bg-white"
            />
          </div>
          <div>
            <label
              htmlFor={`${idPrefix}-end-${index}`}
              className="text-[10px] font-black uppercase tracking-wider text-slate-500 block mb-1"
            >
              À
            </label>
            <input
              id={`${idPrefix}-end-${index}`}
              type="time"
              value={slot.endTime}
              onChange={(e) => updateRow(index, { endTime: e.target.value })}
              className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm bg-white"
            />
          </div>
          <button
            type="button"
            onClick={() => removeRow(index)}
            className="px-3 py-2 rounded-xl border border-slate-200 text-xs font-bold text-slate-600 hover:bg-slate-50"
            aria-label="Retirer ce créneau"
          >
            Retirer
          </button>
        </div>
      ))}
      <button
        type="button"
        onClick={() => onChange([...rows, emptyMakeupSlotDraft()])}
        className="text-xs font-bold text-indigo-700 hover:underline"
      >
        + Ajouter un autre créneau
      </button>
    </div>
  );
}
