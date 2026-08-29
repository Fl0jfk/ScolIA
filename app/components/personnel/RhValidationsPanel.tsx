"use client";

import { useCallback, useEffect, useState } from "react";
import type { PersonnelIndexEntry } from "@/app/lib/personnel-types";

export default function RhValidationsPanel() {
  const [items, setItems] = useState<PersonnelIndexEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [notes, setNotes] = useState<Record<string, string>>({});

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/personnel", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error);
      const index = Array.isArray(j.index) ? (j.index as PersonnelIndexEntry[]) : [];
      setItems(index.filter((e) => e.rhSpaceStatus === "pending_validation"));
    } catch {
      setItems([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const decide = async (personnelId: string, action: "validate" | "reject") => {
    setBusyId(personnelId);
    try {
      const res = await fetch("/api/rh/espace", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action,
          personnelId,
          validationNote: notes[personnelId]?.trim() || undefined,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error);
      await load();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusyId(null);
    }
  };

  if (loading) return <p className="text-sm text-slate-500">Chargement des dossiers…</p>;

  if (items.length === 0) {
    return (
      <div className="rounded-2xl border border-dashed border-slate-300 bg-white p-8 text-center text-slate-500 text-sm">
        Aucun dossier en attente de validation.
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {items.map((r) => (
        <div key={r.id} className="rounded-2xl border border-amber-200 bg-white p-5 shadow-sm">
          <div className="flex flex-wrap items-start justify-between gap-3 mb-3">
            <div>
              <p className="font-black text-slate-900">{r.displayName}</p>
              <p className="text-xs text-slate-500">{r.email}</p>
            </div>
            <span className="text-[10px] font-black uppercase tracking-wide px-2 py-1 rounded-lg bg-amber-50 text-amber-800 border border-amber-200">
              En attente
            </span>
          </div>
          <textarea
            rows={2}
            value={notes[r.id] ?? ""}
            onChange={(e) => setNotes((p) => ({ ...p, [r.id]: e.target.value }))}
            placeholder="Note pour le collaborateur (optionnel)"
            className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm mb-3"
          />
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              disabled={busyId === r.id}
              onClick={() => void decide(r.id, "validate")}
              className="px-4 py-2 rounded-xl bg-emerald-600 text-white text-sm font-bold disabled:opacity-60"
            >
              Valider l&apos;espace RH
            </button>
            <button
              type="button"
              disabled={busyId === r.id}
              onClick={() => void decide(r.id, "reject")}
              className="px-4 py-2 rounded-xl border border-rose-200 text-rose-700 text-sm font-bold disabled:opacity-60"
            >
              Demander des corrections
            </button>
            <a
              href={`/rh/${r.id}`}
              className="px-4 py-2 rounded-xl border border-slate-200 text-slate-700 text-sm font-bold"
            >
              Ouvrir le dossier →
            </a>
          </div>
        </div>
      ))}
    </div>
  );
}
