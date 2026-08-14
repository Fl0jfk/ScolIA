"use client";

import { useMemo, useState } from "react";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

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
    return <p className="text-sm text-slate-500">Chargement des utilisateurs Clerk…</p>;
  }

  return (
    <div className="space-y-2">
      {selected ? (
        <div className="flex items-center justify-between gap-2 rounded-xl border border-indigo-200 bg-indigo-50 px-3 py-2">
          <div className="min-w-0">
            <p className="text-sm font-bold text-slate-800 truncate">{memberLabel(selected)}</p>
            <p className="text-[11px] text-slate-500 truncate">{selected.email}</p>
          </div>
          <button
            type="button"
            className="shrink-0 text-xs font-bold text-rose-600 hover:underline cursor-pointer"
            onClick={() => onChange(null)}
          >
            Retirer
          </button>
        </div>
      ) : (
        <p className="text-xs text-slate-400 italic">Aucune personne sélectionnée.</p>
      )}
      <input
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Rechercher dans le personnel Clerk…"
        className="w-full border rounded-xl px-3 py-2 text-sm"
      />
      <div className="max-h-48 overflow-y-auto border border-slate-200 rounded-xl divide-y bg-white">
        {filtered.length === 0 ? (
          <p className="p-3 text-sm text-slate-400 italic">Aucune personne trouvée.</p>
        ) : (
          filtered.map((m) => {
            const active = m.clerkUserId === selectedId;
            return (
              <button
                key={m.clerkUserId}
                type="button"
                onClick={() => onChange(m)}
                className={`w-full text-left px-3 py-2.5 text-sm cursor-pointer ${
                  active ? "bg-indigo-50" : "hover:bg-slate-50"
                }`}
              >
                <span className="font-semibold text-slate-800">{memberLabel(m)}</span>
                <span className="block text-[11px] text-slate-500 truncate">{m.email}</span>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}
