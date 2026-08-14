"use client";

import {
  isHexSubjectColor,
  isPresetSubjectColor,
  PROF_ROOM_COLOR_PRESETS,
  subjectColorToHex,
} from "@/app/lib/prof-room-subject-colors";
import { dash } from "@/app/lib/dashboard-brand";
import SubjectColorBadge from "./SubjectColorBadge";

const CUSTOM_VALUE = "__custom__";

export default function SubjectColorEditor({
  label,
  value,
  onChange,
  onRemove,
}: {
  label: string;
  value: string;
  onChange: (next: string) => void;
  onRemove?: () => void;
}) {
  const selectValue = isPresetSubjectColor(value) ? value : CUSTOM_VALUE;
  const hex = subjectColorToHex(value);

  return (
    <div className={`flex flex-wrap items-center gap-3 rounded-xl border bg-white/70 p-3 ${dash.borderSoft}`}>
      <SubjectColorBadge label={label} colorValue={value} />
      <select
        value={selectValue}
        onChange={(e) => {
          const v = e.target.value;
          if (v === CUSTOM_VALUE) {
            if (!isHexSubjectColor(value)) onChange(hex);
            return;
          }
          onChange(v);
        }}
        className={`min-w-[140px] flex-1 cursor-pointer rounded-lg border bg-white/80 p-2 text-sm font-semibold ${dash.borderSoft} ${dash.ink}`}
      >
        {PROF_ROOM_COLOR_PRESETS.map((p) => (
          <option key={p.value} value={p.value}>
            {p.label}
          </option>
        ))}
        <option value={CUSTOM_VALUE}>Couleur personnalisée</option>
      </select>
      <label className={`flex shrink-0 items-center gap-2 text-xs font-semibold ${dash.textMid}`}>
        <input
          type="color"
          value={hex}
          onChange={(e) => onChange(e.target.value)}
          className="h-10 w-10 cursor-pointer rounded-lg border border-white/70 bg-white p-0.5"
          title="Choisir une couleur"
        />
        {isHexSubjectColor(value) ? value.toUpperCase() : "Personnalisée"}
      </label>
      {onRemove && (
        <button type="button" onClick={onRemove} className="cursor-pointer text-xs font-semibold text-rose-700">
          Supprimer
        </button>
      )}
    </div>
  );
}
