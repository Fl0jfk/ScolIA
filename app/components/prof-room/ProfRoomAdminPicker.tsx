"use client";

import { useMemo, useState } from "react";
import { dash } from "@/app/lib/dashboard-brand";

export type ClerkMemberOption = {
  clerkUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  pending?: boolean;
};

function memberLabel(m: ClerkMemberOption): string {
  const name = m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim();
  return name || m.email;
}

export default function ProfRoomAdminPicker({
  members,
  selectedIds,
  onChange,
  loading,
  footerHint,
}: {
  members: ClerkMemberOption[];
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  loading?: boolean;
  footerHint?: string;
}) {
  const [search, setSearch] = useState("");

  const activeMembers = useMemo(
    () => members.filter((m) => m.clerkUserId && !m.pending),
    [members],
  );

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return activeMembers;
    return activeMembers.filter((m) => {
      const hay = `${memberLabel(m)} ${m.email} ${m.lastName ?? ""}`.toLowerCase();
      return hay.includes(q);
    });
  }, [activeMembers, search]);

  const toggle = (id: string) => {
    if (selectedIds.includes(id)) onChange(selectedIds.filter((x) => x !== id));
    else onChange([...selectedIds, id]);
  };

  if (loading) {
    return <p className={`text-sm ${dash.textMid}`}>Chargement des utilisateurs Clerk…</p>;
  }

  return (
    <div className="space-y-3">
      <input
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Rechercher une personne…"
        className={`w-full rounded-xl border bg-white/80 px-3 py-2.5 text-sm font-semibold outline-none ${dash.borderSoft} ${dash.ink} ${dash.focusBorder}`}
      />
      <div className={`max-h-72 divide-y overflow-y-auto rounded-xl border bg-white/70 ${dash.borderSoft} ${dash.divider}`}>
        {filtered.length === 0 ? (
          <p className={`p-4 text-sm italic ${dash.textMid}`}>Aucune personne trouvée.</p>
        ) : (
          filtered.map((m) => (
            <label
              key={m.clerkUserId}
              className={`flex cursor-pointer items-center gap-3 px-4 py-3 text-sm ${dash.hoverBgSoft}`}
            >
              <input
                type="checkbox"
                checked={selectedIds.includes(m.clerkUserId)}
                onChange={() => toggle(m.clerkUserId)}
                className="rounded border-slate-300"
              />
              <span className="flex-1 min-w-0">
                <span className={`block truncate font-semibold ${dash.ink}`}>{memberLabel(m)}</span>
                <span className={`block truncate text-xs ${dash.textMid}`}>{m.email}</span>
                {m.lastName && (
                  <span className={`text-[10px] uppercase tracking-wide ${dash.textMid}`}>
                    Nom de famille utilisé : {m.lastName.toUpperCase()}
                  </span>
                )}
              </span>
            </label>
          ))
        )}
      </div>
      <p className={`text-xs ${dash.textMid}`}>
        {footerHint ??
          `${selectedIds.length} administrateur(s) sélectionné(s). Le mode admin du planning utilise le nom de famille Clerk de chaque personne.`}
      </p>
    </div>
  );
}
