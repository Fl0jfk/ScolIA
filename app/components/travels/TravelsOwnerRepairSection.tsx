"use client";

import { useEffect, useMemo, useState } from "react";
import type { DirectoryAssigneeOption } from "@/app/components/domain-planning/DomainAssigneePicker";
import TravelsTeacherPicker from "@/app/components/travels/TravelsTeacherPicker";
import type { TravelsTrip } from "@/app/lib/travels-types";

type Props = {
  trip: TravelsTrip;
  onRepaired: (trip: TravelsTrip) => void;
};

export default function TravelsOwnerRepairSection({ trip, onRepaired }: Props) {
  const [users, setUsers] = useState<DirectoryAssigneeOption[]>([]);
  const [loadingUsers, setLoadingUsers] = useState(true);
  const [selectedId, setSelectedId] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    fetch("/api/travels/directory-users")
      .then((res) => res.json())
      .then((data) => {
        if (!cancelled) setUsers(Array.isArray(data.users) ? data.users : []);
      })
      .catch(() => {
        if (!cancelled) setUsers([]);
      })
      .finally(() => {
        if (!cancelled) setLoadingUsers(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const selected = useMemo(
    () => users.find((u) => u.externalUserId === selectedId) ?? null,
    [users, selectedId],
  );

  async function repairOwner() {
    if (!selected) return;
    const ownerName =
      selected.displayName ||
      `${selected.firstName ?? ""} ${selected.lastName ?? ""}`.trim() ||
      selected.email;
    if (
      !confirm(
        `Réattribuer ce dossier à ${ownerName} ?\n\nLe créateur affiché sera corrigé définitivement.`,
      )
    ) {
      return;
    }
    setSaving(true);
    try {
      const updatedTrip: TravelsTrip = {
        ...trip,
        ownerId: selected.externalUserId,
        ownerName,
        ownerEmail: selected.email,
        history: [
          ...(trip.history || []),
          {
            date: new Date().toISOString(),
            user: "Administratif",
            action: "OWNER_REASSIGNED",
            note: `Créateur corrigé : ${ownerName}`,
          },
        ],
      };
      const res = await fetch("/api/travels/update", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: trip.id, data: updatedTrip }),
      });
      if (!res.ok) {
        const payload = await res.json().catch(() => ({}));
        throw new Error(payload?.error || "Correction impossible.");
      }
      onRepaired(updatedTrip);
      setSelectedId("");
      alert(`Créateur corrigé : ${ownerName}`);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur lors de la correction.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="mb-6 rounded-2xl border border-amber-200 bg-gradient-to-br from-amber-50 to-orange-50/60 p-5 space-y-3 shadow-sm">
      <div className="flex items-start gap-3">
        <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-amber-100 text-lg">
          👤
        </span>
        <div>
          <p className="text-sm font-black text-amber-950">Correction du créateur du dossier</p>
          <p className="text-xs text-amber-800/90 mt-1 leading-relaxed">
            Créateur actuel : <strong>{trip.ownerName || "—"}</strong>
            {trip.ownerId ? ` (${trip.ownerId})` : ""}. À utiliser si le nom a été écrasé par une
            modification antérieure.
          </p>
        </div>
      </div>
      <TravelsTeacherPicker
        users={users}
        value={selectedId}
        loading={loadingUsers}
        disabled={saving}
        onChange={(u) => setSelectedId(u?.externalUserId || "")}
      />
      <button
        type="button"
        disabled={!selected || saving}
        onClick={() => void repairOwner()}
        className="rounded-xl bg-amber-700 px-4 py-2 text-sm font-black text-white disabled:opacity-50 hover:bg-amber-800"
      >
        {saving ? "Enregistrement…" : "Corriger le créateur"}
      </button>
    </div>
  );
}
