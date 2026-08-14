"use client";

import { useMemo, useState } from "react";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import { settingsInputClass } from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

function memberLabel(m: ClerkMemberOption): string {
  const name = m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim();
  return name || m.email;
}

export default function ClerkPersonSelect({
  members,
  selectedId,
  onChange,
  loading,
}: {
  members: ClerkMemberOption[];
  selectedId: string;
  onChange: (member: ClerkMemberOption | null) => void;
  loading?: boolean;
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

  const selected = activeMembers.find((m) => m.clerkUserId === selectedId);

  if (loading) {
    return <p className={`text-sm ${dash.textMid}`}>Chargement des utilisateurs Clerk…</p>;
  }

  return (
    <div className="space-y-2">
      {selected ? (
        <div className="flex items-center justify-between gap-2 rounded-2xl border border-[color:var(--dash-primary)]/25 bg-[color:var(--dash-soft)]/70 px-3 py-2">
          <div className="min-w-0">
            <p className={`truncate text-sm font-semibold ${dash.ink}`}>{memberLabel(selected)}</p>
            <p className={`truncate text-[11px] ${dash.textMid}`}>{selected.email}</p>
          </div>
          <button
            type="button"
            className="shrink-0 cursor-pointer text-xs font-semibold text-rose-600 hover:underline"
            onClick={() => onChange(null)}
          >
            Retirer
          </button>
        </div>
      ) : (
        <p className={`text-xs italic ${dash.textMid}`}>Aucune personne sélectionnée.</p>
      )}
      <input
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Rechercher dans le personnel Clerk…"
        className={settingsInputClass}
      />
      <div className="max-h-48 divide-y divide-white/60 overflow-y-auto rounded-2xl border border-white/70 bg-white/60 backdrop-blur-sm">
        {filtered.length === 0 ? (
          <p className={`p-3 text-sm italic ${dash.textMid}`}>Aucune personne trouvée.</p>
        ) : (
          filtered.map((m) => {
            const active = m.clerkUserId === selectedId;
            return (
              <button
                key={m.clerkUserId}
                type="button"
                onClick={() => onChange(m)}
                className={`w-full cursor-pointer px-3 py-2.5 text-left text-sm ${
                  active ? "bg-[color:var(--dash-soft)]/80" : "hover:bg-white/80"
                }`}
              >
                <span className={`font-semibold ${dash.ink}`}>{memberLabel(m)}</span>
                <span className={`block truncate text-[11px] ${dash.textMid}`}>{m.email}</span>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}
