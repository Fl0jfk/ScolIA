"use client";

import { useMemo, useState } from "react";
import {
  parseClassesSelection,
  serializeClassesSelection,
  TRAVELS_CLASSES_AUTRES_LABEL,
} from "@/app/lib/travels-classes";

export default function TripClassesMultiSelect({
  value,
  options,
  onChange,
  required,
  disabled,
  id,
}: {
  value: string;
  options: string[];
  onChange: (next: string) => void;
  required?: boolean;
  disabled?: boolean;
  id?: string;
}) {
  const parsed = useMemo(() => parseClassesSelection(value, options), [value, options]);
  const [autresOpen, setAutresOpen] = useState(parsed.autres || Boolean(parsed.otherText));

  const toggleClass = (cls: string) => {
    const nextSelected = parsed.selected.includes(cls)
      ? parsed.selected.filter((c) => c !== cls)
      : [...parsed.selected, cls];
    onChange(serializeClassesSelection(nextSelected, parsed.otherText));
  };

  const setOtherText = (text: string) => {
    onChange(serializeClassesSelection(parsed.selected, text));
  };

  const toggleAutres = () => {
    if (autresOpen) {
      setAutresOpen(false);
      onChange(serializeClassesSelection(parsed.selected, ""));
      return;
    }
    setAutresOpen(true);
  };

  if (options.length === 0) {
    return (
      <div className="space-y-1">
        <input
          id={id}
          required={required}
          disabled={disabled}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="w-full p-3 bg-slate-50 border rounded-xl outline-indigo-500"
          placeholder="Ex: 3A, 4B"
        />
        <p className="text-[11px] text-amber-700">
          Catalogue vide : renseignez les classes dans Paramètres (salles / enseignements) pour
          les proposer ici en sélection.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-1.5 max-h-44 overflow-y-auto rounded-xl border border-slate-200 bg-slate-50 p-2.5">
        {options.map((cls) => {
          const on = parsed.selected.includes(cls);
          return (
            <button
              key={cls}
              type="button"
              disabled={disabled}
              onClick={() => toggleClass(cls)}
              className={`rounded-lg px-2.5 py-1.5 text-xs font-bold border transition ${
                on
                  ? "bg-indigo-600 text-white border-indigo-600"
                  : "bg-white text-slate-700 border-slate-200 hover:border-indigo-300"
              } disabled:opacity-50`}
            >
              {cls}
            </button>
          );
        })}
        <button
          type="button"
          disabled={disabled}
          onClick={toggleAutres}
          className={`rounded-lg px-2.5 py-1.5 text-xs font-bold border transition ${
            autresOpen || parsed.autres
              ? "bg-amber-500 text-white border-amber-500"
              : "bg-white text-slate-700 border-slate-200 hover:border-amber-300"
          } disabled:opacity-50`}
        >
          {TRAVELS_CLASSES_AUTRES_LABEL}
        </button>
      </div>
      {(autresOpen || parsed.autres) && (
        <input
          type="text"
          disabled={disabled}
          value={parsed.otherText}
          onChange={(e) => setOtherText(e.target.value)}
          placeholder="Précisez les autres classes…"
          className="w-full p-2.5 bg-amber-50 border border-amber-200 rounded-xl outline-indigo-500 text-sm"
        />
      )}
      {required && !value.trim() ? (
        <p className="text-[11px] text-rose-600 font-semibold">Sélectionnez au moins une classe (ou Autres).</p>
      ) : null}
    </div>
  );
}
