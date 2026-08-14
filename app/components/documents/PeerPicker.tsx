"use client";

import { peerFullName, peerRoleLabels, type Peer } from "@/app/lib/documents-page-model";

export default function PeerPicker({
  peers,
  search,
  onSearch,
  selected,
  onToggle,
}: {
  peers: Peer[];
  search: string;
  onSearch: (v: string) => void;
  selected: string[];
  onToggle: (id: string) => void;
}) {
  return (
    <div>
      <input
        value={search}
        onChange={(e) => onSearch(e.target.value)}
        placeholder="Rechercher une personne…"
        className="w-full border border-gray-300 rounded-xl px-3 py-2 text-sm mb-2"
      />
      <div className="max-h-52 overflow-y-auto border border-gray-200 rounded-xl divide-y">
        {peers.length === 0 ? (
          <p className="p-3 text-sm text-gray-400 italic">Aucune personne trouvée.</p>
        ) : (
          peers.map((p) => (
            <label key={p.clerkUserId} className="flex items-center gap-3 px-3 py-2 hover:bg-gray-50 cursor-pointer text-sm">
              <input
                type="checkbox"
                checked={selected.includes(p.clerkUserId)}
                onChange={() => onToggle(p.clerkUserId)}
              />
              <span className="flex-1 min-w-0">
                <span className="font-medium text-gray-800 block truncate">{peerFullName(p)}</span>
                <span className="text-xs text-gray-500 truncate block">{peerRoleLabels(p)}</span>
              </span>
            </label>
          ))
        )}
      </div>
      <p className="text-xs text-gray-400 mt-2">{selected.length} personne(s) sélectionnée(s)</p>
    </div>
  );
}
