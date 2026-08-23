"use client";

import { useMemo, useState } from "react";
import type { DirectoryAssigneeOption } from "@/app/components/domain-planning/DomainAssigneePicker";

function norm(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

function labelOf(u: DirectoryAssigneeOption): string {
  const name = u.displayName || `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim();
  return name || u.email;
}

type Props = {
  users: DirectoryAssigneeOption[];
  value: string;
  loading?: boolean;
  disabled?: boolean;
  onChange: (user: DirectoryAssigneeOption | null) => void;
};

/** Sélecteur enseignant — panneau inline (pas de position absolute, évite le clipping). */
export default function TravelsTeacherPicker({
  users,
  value,
  loading,
  disabled,
  onChange,
}: Props) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");

  const activeUsers = useMemo(
    () => users.filter((u) => u.externalUserId && !u.pending),
    [users],
  );

  const selected = activeUsers.find((u) => u.externalUserId === value) ?? null;

  const filtered = useMemo(() => {
    const q = norm(search.trim());
    if (!q) return activeUsers;
    return activeUsers.filter((u) => {
      const hay = norm(`${labelOf(u)} ${u.email} ${u.firstName ?? ""} ${u.lastName ?? ""}`);
      return hay.includes(q);
    });
  }, [activeUsers, search]);

  if (loading) {
    return <p className="text-sm text-amber-900/70">Chargement des utilisateurs…</p>;
  }

  return (
    <div className="space-y-2">
      <button
        type="button"
        disabled={disabled}
        onClick={() => setOpen((o) => !o)}
        className="w-full text-left rounded-xl border border-amber-200 bg-white px-4 py-3 text-sm font-bold text-slate-800 shadow-sm hover:border-amber-400 transition-colors disabled:opacity-50"
      >
        {selected ? (
          <span>
            {labelOf(selected)}
            <span className="block text-xs font-normal text-slate-500 mt-0.5">{selected.email}</span>
          </span>
        ) : (
          <span className="text-slate-500">Choisir un enseignant…</span>
        )}
      </button>

      {selected && !disabled && (
        <button
          type="button"
          onClick={() => {
            onChange(null);
            setSearch("");
          }}
          className="text-xs font-bold text-amber-800 hover:text-amber-950"
        >
          Effacer la sélection
        </button>
      )}

      {open && !disabled && (
        <div className="rounded-xl border border-amber-200 bg-white shadow-lg">
          <div className="p-3 border-b border-amber-100">
            <input
              autoFocus
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Rechercher par nom ou e-mail…"
              className="w-full rounded-lg border border-slate-200 bg-slate-50 px-3 py-2.5 text-sm text-slate-900 placeholder:text-slate-400 outline-none focus:ring-2 focus:ring-amber-200"
            />
          </div>
          <ul className="max-h-72 overflow-y-auto divide-y divide-slate-100">
            {filtered.length === 0 ? (
              <li className="p-4 text-sm text-slate-500 italic">Aucun utilisateur trouvé.</li>
            ) : (
              filtered.map((u) => (
                <li key={u.externalUserId}>
                  <button
                    type="button"
                    className={`w-full text-left px-4 py-3 hover:bg-amber-50 transition-colors ${
                      u.externalUserId === value ? "bg-amber-100/80" : ""
                    }`}
                    onClick={() => {
                      onChange(u);
                      setOpen(false);
                      setSearch("");
                    }}
                  >
                    <span className="block text-sm font-bold text-slate-900">{labelOf(u)}</span>
                    <span className="block text-xs text-slate-500">{u.email}</span>
                  </button>
                </li>
              ))
            )}
          </ul>
        </div>
      )}
    </div>
  );
}
