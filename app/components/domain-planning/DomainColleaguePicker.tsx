"use client";

import { useMemo, useState } from "react";
import type { DirectoryAssigneeOption } from "@/app/components/domain-planning/DomainAssigneePicker";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";

export type ColleagueScopeFilter = "all" | "professeurs" | "personnels";

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

/** Compte dont le seul rôle est professeur. */
function isProfesseurOnly(roles: string[] | undefined): boolean {
  const normalized = normalizeIntranetRoles(roles || []);
  return normalized.length > 0 && normalized.every((r) => r === "professeur");
}

function hasProfesseurRole(roles: string[] | undefined): boolean {
  return normalizeIntranetRoles(roles || []).includes("professeur");
}

/** Personnel = tout compte qui n'est pas « professeur only ». */
function isPersonnel(roles: string[] | undefined): boolean {
  return !isProfesseurOnly(roles);
}

function sortByLastThenFirst(a: DirectoryAssigneeOption, b: DirectoryAssigneeOption): number {
  const lastA = norm(a.lastName || "");
  const lastB = norm(b.lastName || "");
  if (lastA !== lastB) return lastA.localeCompare(lastB, "fr");
  const firstA = norm(a.firstName || "");
  const firstB = norm(b.firstName || "");
  if (firstA !== firstB) return firstA.localeCompare(firstB, "fr");
  return norm(labelOf(a)).localeCompare(norm(labelOf(b)), "fr");
}

type Props = {
  users: DirectoryAssigneeOption[];
  value: string;
  scope: ColleagueScopeFilter;
  loading?: boolean;
  disabled?: boolean;
  excludeUserId?: string;
  onChange: (user: DirectoryAssigneeOption | null) => void;
  onScopeChange: (scope: ColleagueScopeFilter) => void;
};

/** Sélecteur collègue (profs / personnels) — tri nom puis prénom. */
export default function DomainColleaguePicker({
  users,
  value,
  scope,
  loading,
  disabled,
  excludeUserId,
  onChange,
  onScopeChange,
}: Props) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");

  const scopedUsers = useMemo(() => {
    const active = users.filter(
      (u) => u.externalUserId && !u.pending && u.externalUserId !== excludeUserId,
    );
    const filtered =
      scope === "professeurs"
        ? active.filter((u) => hasProfesseurRole(u.roles))
        : scope === "personnels"
          ? active.filter((u) => isPersonnel(u.roles))
          : active;
    return [...filtered].sort(sortByLastThenFirst);
  }, [users, scope, excludeUserId]);

  const selected = scopedUsers.find((u) => u.externalUserId === value)
    ?? users.find((u) => u.externalUserId === value && u.externalUserId !== excludeUserId)
    ?? null;

  const filtered = useMemo(() => {
    const q = norm(search.trim());
    if (!q) return scopedUsers;
    return scopedUsers.filter((u) => {
      const hay = norm(`${labelOf(u)} ${u.email} ${u.firstName ?? ""} ${u.lastName ?? ""}`);
      return hay.includes(q);
    });
  }, [scopedUsers, search]);

  if (loading) {
    return <p className="text-sm text-slate-500">Chargement de l&apos;annuaire…</p>;
  }

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2" role="group" aria-label="Filtrer l'annuaire">
        {(
          [
            { id: "all", label: "Tous" },
            { id: "professeurs", label: "Professeurs" },
            { id: "personnels", label: "Personnels" },
          ] as const
        ).map((opt) => (
          <button
            key={opt.id}
            type="button"
            disabled={disabled}
            onClick={() => {
              onScopeChange(opt.id);
              if (selected && opt.id !== scope) {
                const stillVisible =
                  opt.id === "all"
                    ? true
                    : opt.id === "professeurs"
                      ? hasProfesseurRole(selected.roles)
                      : isPersonnel(selected.roles);
                if (!stillVisible) onChange(null);
              }
            }}
            className={`rounded-lg px-3 py-1.5 text-xs font-black uppercase transition-colors disabled:opacity-50 ${
              scope === opt.id
                ? "bg-violet-600 text-white"
                : "border border-slate-200 bg-white text-slate-700 hover:border-violet-300"
            }`}
          >
            {opt.label}
          </button>
        ))}
      </div>

      <button
        type="button"
        disabled={disabled}
        onClick={() => setOpen((o) => !o)}
        className="w-full text-left rounded-xl border border-slate-200 bg-white px-4 py-3 text-sm font-bold text-slate-800 shadow-sm hover:border-violet-400 transition-colors disabled:opacity-50"
      >
        {selected ? (
          <span>
            {labelOf(selected)}
            <span className="block text-xs font-normal text-slate-500 mt-0.5">{selected.email}</span>
          </span>
        ) : (
          <span className="text-slate-500">Choisir une personne…</span>
        )}
      </button>

      {selected && !disabled && (
        <button
          type="button"
          onClick={() => {
            onChange(null);
            setSearch("");
          }}
          className="text-xs font-bold text-violet-800 hover:text-violet-950"
        >
          Effacer la sélection
        </button>
      )}

      {open && !disabled && (
        <div className="rounded-xl border border-slate-200 bg-white shadow-lg">
          <div className="p-3 border-b border-slate-100">
            <input
              autoFocus
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Rechercher par nom ou e-mail…"
              className="w-full rounded-lg border border-slate-200 bg-slate-50 px-3 py-2.5 text-sm text-slate-900 placeholder:text-slate-400 outline-none focus:ring-2 focus:ring-violet-200"
            />
          </div>
          <ul className="max-h-72 overflow-y-auto divide-y divide-slate-100">
            {filtered.length === 0 ? (
              <li className="p-4 text-sm text-slate-500 italic">Aucune personne trouvée.</li>
            ) : (
              filtered.map((u) => (
                <li key={u.externalUserId}>
                  <button
                    type="button"
                    className={`w-full text-left px-4 py-3 hover:bg-violet-50 transition-colors ${
                      u.externalUserId === value ? "bg-violet-100/80" : ""
                    }`}
                    onClick={() => {
                      onChange(u);
                      setOpen(false);
                      setSearch("");
                    }}
                  >
                    <span className="block text-sm font-bold text-slate-900">
                      {[u.lastName, u.firstName].filter(Boolean).join(" ").trim() || labelOf(u)}
                    </span>
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
