"use client";

import { useAppContext } from "@/app/hooks/useAppContext";
import type { Establishment, EstablishmentKind } from "@/app/lib/app-config-schemas";
import { establishmentSelectOptions } from "@/app/lib/establishment-catalog";

export default function EstablishmentSelect({
  value,
  onChange,
  establishments,
  includeGroupe = false,
  kinds,
  emptyLabel = "— Choisir —",
  className = "w-full border rounded-xl px-3 py-2 text-sm bg-white",
  required,
  disabled,
  id,
  name,
}: {
  value: string;
  onChange: (label: string) => void;
  establishments?: Establishment[];
  includeGroupe?: boolean;
  kinds?: EstablishmentKind[];
  emptyLabel?: string;
  className?: string;
  required?: boolean;
  disabled?: boolean;
  id?: string;
  name?: string;
}) {
  const { data } = useAppContext();
  const list = establishments ?? data?.establishments ?? [];
  const options = establishmentSelectOptions(list, { includeGroupe, kinds });
  return (
    <select
      id={id}
      name={name}
      className={className}
      value={value}
      required={required}
      disabled={disabled || options.length === 0}
      onChange={(e) => onChange(e.target.value)}
    >
      <option value="">{options.length === 0 ? "Aucun établissement configuré" : emptyLabel}</option>
      {options.map((o) => (
        <option key={o.id} value={o.label}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
