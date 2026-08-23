"use client";

import { useMemo, useState } from "react";
import {
  comparePeersByName,
  peerFullName,
  peerMatchesQuery,
  peerRoleFilters,
  peerRoleLabels,
  type Peer,
  type PeerSortKey,
} from "@/app/lib/documents-page-model";

function formatPeerName(p: Peer, sortBy: PeerSortKey): string {
  const first = (p.firstName ?? "").trim();
  const last = (p.lastName ?? "").trim();
  if (!first && !last) return peerFullName(p);
  if (sortBy === "lastName") return [last, first].filter(Boolean).join(" ");
  return [first, last].filter(Boolean).join(" ");
}

export default function PeerPicker({
  peers,
  selected,
  onChangeSelected,
}: {
  peers: Peer[];
  selected: string[];
  onChangeSelected: (ids: string[]) => void;
}) {
  const [search, setSearch] = useState("");
  const [roleFilter, setRoleFilter] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<PeerSortKey>("lastName");

  const roleOptions = useMemo(() => peerRoleFilters(peers), [peers]);

  const visiblePeers = useMemo(() => {
    const filtered = peers.filter((p) => {
      if (!peerMatchesQuery(p, search)) return false;
      if (roleFilter && !p.roles.includes(roleFilter)) return false;
      return true;
    });
    return filtered.sort((a, b) => comparePeersByName(a, b, sortBy));
  }, [peers, search, roleFilter, sortBy]);

  const visibleIds = visiblePeers.map((p) => p.externalUserId);
  const visibleSet = new Set(visibleIds);
  const allVisibleSelected = visibleIds.length > 0 && visibleIds.every((id) => selected.includes(id));

  const toggleOne = (id: string) => {
    onChangeSelected(selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id]);
  };

  const toggleVisible = () => {
    if (allVisibleSelected) {
      onChangeSelected(selected.filter((id) => !visibleSet.has(id)));
      return;
    }
    const kept = selected.filter((id) => !visibleSet.has(id));
    onChangeSelected([...kept, ...visibleIds]);
  };

  return (
    <div>
      <input
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Rechercher une personne…"
        className="w-full border border-gray-300 rounded-xl px-3 py-2 text-sm mb-2"
      />

      {roleOptions.length > 0 && (
        <div className="mb-2">
          <p className="text-[11px] font-semibold uppercase tracking-wide text-gray-400 mb-1.5">
            Type de personnel
          </p>
          <div className="flex flex-wrap gap-1.5">
            <RoleChip
              label="Tous"
              count={peers.length}
              active={roleFilter === null}
              onClick={() => setRoleFilter(null)}
            />
            {roleOptions.map((opt) => (
              <RoleChip
                key={opt.slug}
                label={opt.label}
                count={opt.count}
                active={roleFilter === opt.slug}
                onClick={() => setRoleFilter(roleFilter === opt.slug ? null : opt.slug)}
              />
            ))}
          </div>
        </div>
      )}

      <div className="flex items-center justify-between gap-2 mb-2">
        <div className="flex items-center gap-1.5 text-xs text-gray-500">
          <span>Trier par</span>
          <SortButton active={sortBy === "lastName"} onClick={() => setSortBy("lastName")}>
            Nom
          </SortButton>
          <SortButton active={sortBy === "firstName"} onClick={() => setSortBy("firstName")}>
            Prénom
          </SortButton>
        </div>
        <button
          type="button"
          disabled={visiblePeers.length === 0}
          onClick={toggleVisible}
          className="text-xs font-semibold text-blue-700 hover:text-blue-900 disabled:text-gray-300 disabled:cursor-default"
        >
          {allVisibleSelected ? "Tout désélectionner" : "Tout sélectionner"}
        </button>
      </div>

      <div className="max-h-64 overflow-y-auto border border-gray-200 rounded-xl divide-y">
        {visiblePeers.length === 0 ? (
          <p className="p-3 text-sm text-gray-400 italic">Aucune personne trouvée.</p>
        ) : (
          visiblePeers.map((p) => (
            <label
              key={p.externalUserId}
              className="flex items-center gap-3 px-3 py-2 hover:bg-gray-50 cursor-pointer text-sm"
            >
              <input
                type="checkbox"
                checked={selected.includes(p.externalUserId)}
                onChange={() => toggleOne(p.externalUserId)}
              />
              <span className="flex-1 min-w-0">
                <span className="font-medium text-gray-800 block truncate">{formatPeerName(p, sortBy)}</span>
                <span className="text-xs text-gray-500 truncate block">{peerRoleLabels(p)}</span>
              </span>
            </label>
          ))
        )}
      </div>
      <p className="text-xs text-gray-400 mt-2">
        {visiblePeers.length} affichée{visiblePeers.length > 1 ? "s" : ""}
        {roleFilter ? ` · filtre actif` : ""} · {selected.length} sélectionnée
        {selected.length > 1 ? "s" : ""}
      </p>
    </div>
  );
}

function RoleChip({
  label,
  count,
  active,
  onClick,
}: {
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-xs font-semibold border transition ${
        active
          ? "bg-blue-600 text-white border-blue-600"
          : "bg-white text-gray-600 border-gray-200 hover:border-blue-300 hover:text-blue-700"
      }`}
    >
      {label}
      <span className={active ? "text-blue-100" : "text-gray-400"}>{count}</span>
    </button>
  );
}

function SortButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded-lg px-2 py-0.5 text-xs font-semibold border ${
        active ? "bg-slate-800 text-white border-slate-800" : "bg-white text-gray-600 border-gray-200 hover:border-slate-400"
      }`}
    >
      {children}
    </button>
  );
}
