"use client";

import { useMemo, useState } from "react";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import { settingsInputClass } from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

export function directoryMemberLabel(m: DirectoryMemberOption): string {
  const name = m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim();
  return name || m.email;
}

function normEmail(value: string | undefined | null): string {
  return String(value || "").trim().toLowerCase();
}

export function findDirectoryMember(
  members: DirectoryMemberOption[],
  opts: { id?: string; email?: string },
): DirectoryMemberOption | undefined {
  const id = String(opts.id || "").trim();
  if (id) {
    const byId = members.find((m) => m.externalUserId === id);
    if (byId) return byId;
  }
  const email = normEmail(opts.email);
  if (!email) return undefined;
  return members.find((m) => normEmail(m.email) === email);
}

function SelectedChip({
  name,
  email,
  unmatched,
  onRemove,
}: {
  name: string;
  email: string;
  unmatched?: boolean;
  onRemove: () => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2 rounded-2xl border border-[color:var(--dash-primary)]/25 bg-[color:var(--dash-soft)]/70 px-3 py-2">
      <div className="min-w-0">
        <p className={`truncate text-sm font-semibold ${dash.ink}`}>{name}</p>
        <p className={`truncate text-[11px] ${dash.textMid}`}>
          {email}
          {unmatched ? " — hors personnel" : ""}
        </p>
      </div>
      <button
        type="button"
        className="shrink-0 cursor-pointer text-xs font-semibold text-rose-600 hover:underline"
        onClick={onRemove}
      >
        Retirer
      </button>
    </div>
  );
}

function MemberSearchList({
  members,
  selectedIds,
  onPick,
}: {
  members: DirectoryMemberOption[];
  selectedIds: Set<string>;
  onPick: (member: DirectoryMemberOption) => void;
}) {
  const [search, setSearch] = useState("");
  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return members;
    return members.filter((m) => {
      const hay = `${directoryMemberLabel(m)} ${m.email} ${m.lastName ?? ""}`.toLowerCase();
      return hay.includes(q);
    });
  }, [members, search]);

  return (
    <div className="space-y-2">
      <input
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Rechercher dans le personnel…"
        className={`${settingsInputClass} mt-0`}
      />
      <div className="max-h-44 divide-y divide-white/60 overflow-y-auto rounded-2xl border border-white/70 bg-white/60 backdrop-blur-sm">
        {filtered.length === 0 ? (
          <p className={`p-3 text-sm italic ${dash.textMid}`}>Aucune personne trouvée.</p>
        ) : (
          filtered.map((m) => {
            const active = selectedIds.has(m.externalUserId);
            return (
              <button
                key={m.externalUserId}
                type="button"
                onClick={() => onPick(m)}
                className={`w-full cursor-pointer px-3 py-2.5 text-left text-sm ${
                  active ? "bg-[color:var(--dash-soft)]/80" : "hover:bg-white/80"
                }`}
              >
                <span className={`font-semibold ${dash.ink}`}>{directoryMemberLabel(m)}</span>
                <span className={`block truncate text-[11px] ${dash.textMid}`}>{m.email}</span>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}

export default function DirectoryPersonSelect({
  members,
  selectedId,
  selectedEmail,
  onChange,
  loading,
}: {
  members: DirectoryMemberOption[];
  selectedId?: string;
  selectedEmail?: string;
  onChange: (member: DirectoryMemberOption | null) => void;
  loading?: boolean;
}) {
  const activeMembers = useMemo(
    () => members.filter((m) => m.externalUserId && !m.pending),
    [members],
  );
  const selected = findDirectoryMember(activeMembers, { id: selectedId, email: selectedEmail });
  const unmatched = !selected && Boolean(normEmail(selectedEmail));
  const hasSelection = Boolean(selected || unmatched);
  const [picking, setPicking] = useState(!hasSelection);

  if (loading) {
    return <p className={`text-sm ${dash.textMid}`}>Chargement des utilisateurs du directory…</p>;
  }

  return (
    <div className="space-y-2">
      {selected ? (
        <SelectedChip
          name={directoryMemberLabel(selected)}
          email={selected.email}
          onRemove={() => {
            onChange(null);
            setPicking(true);
          }}
        />
      ) : unmatched ? (
        <SelectedChip
          name={String(selectedEmail)}
          email={String(selectedEmail)}
          unmatched
          onRemove={() => {
            onChange(null);
            setPicking(true);
          }}
        />
      ) : (
        <p className={`text-xs italic ${dash.textMid}`}>Aucune personne sélectionnée.</p>
      )}
      {hasSelection && !picking ? (
        <button
          type="button"
          className={`text-xs font-semibold ${dash.textPrimary} hover:underline`}
          onClick={() => setPicking(true)}
        >
          Changer
        </button>
      ) : (
        <MemberSearchList
          members={activeMembers}
          selectedIds={new Set(selected ? [selected.externalUserId] : [])}
          onPick={(member) => {
            onChange(member);
            setPicking(false);
          }}
        />
      )}
    </div>
  );
}

export function DirectoryPeopleSelect({
  members,
  selectedEmails,
  onChange,
  loading,
}: {
  members: DirectoryMemberOption[];
  selectedEmails: string[];
  onChange: (emails: string[]) => void;
  loading?: boolean;
}) {
  const activeMembers = useMemo(
    () => members.filter((m) => m.externalUserId && !m.pending),
    [members],
  );
  const selectedSet = useMemo(
    () => new Set(selectedEmails.map(normEmail).filter(Boolean)),
    [selectedEmails],
  );
  const matched = activeMembers.filter((m) => selectedSet.has(normEmail(m.email)));
  const unmatched = selectedEmails.filter(
    (email) => email.trim() && !activeMembers.some((m) => normEmail(m.email) === normEmail(email)),
  );
  const selectedIds = new Set(matched.map((m) => m.externalUserId));

  if (loading) {
    return <p className={`text-sm ${dash.textMid}`}>Chargement des utilisateurs du directory…</p>;
  }

  const emit = (nextEmails: string[]) => {
    const seen = new Set<string>();
    onChange(
      nextEmails.filter((e) => {
        const k = normEmail(e);
        if (!k || seen.has(k)) return false;
        seen.add(k);
        return true;
      }),
    );
  };

  return (
    <div className="space-y-2">
      {matched.length === 0 && unmatched.length === 0 ? (
        <p className={`text-xs italic ${dash.textMid}`}>Aucune personne sélectionnée.</p>
      ) : (
        <div className="space-y-1.5">
          {matched.map((m) => (
            <SelectedChip
              key={m.externalUserId}
              name={directoryMemberLabel(m)}
              email={m.email}
              onRemove={() => emit(selectedEmails.filter((e) => normEmail(e) !== normEmail(m.email)))}
            />
          ))}
          {unmatched.map((email) => (
            <SelectedChip
              key={email}
              name={email}
              email={email}
              unmatched
              onRemove={() => emit(selectedEmails.filter((e) => normEmail(e) !== normEmail(email)))}
            />
          ))}
        </div>
      )}
      <MemberSearchList
        members={activeMembers}
        selectedIds={selectedIds}
        onPick={(member) => {
          const email = member.email.trim();
          if (!email) return;
          if (selectedSet.has(normEmail(email))) {
            emit(selectedEmails.filter((e) => normEmail(e) !== normEmail(email)));
            return;
          }
          emit([...selectedEmails, email]);
        }}
      />
    </div>
  );
}
