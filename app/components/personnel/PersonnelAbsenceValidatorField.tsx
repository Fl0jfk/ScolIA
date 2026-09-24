"use client";

import { useEffect, useState } from "react";
import DirectoryPersonSelect, {
  directoryMemberLabel,
} from "@/app/components/settings/DirectoryPersonSelect";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import {
  parsePersonnelAbsenceManager,
  serializePersonnelAbsenceManager,
} from "@/app/lib/absences-ogec-validators-shared";

type Props = {
  managerId: string | null | undefined;
  disabled?: boolean;
  onSave: (managerId: string | null) => Promise<void>;
};

export default function PersonnelAbsenceValidatorField({ managerId, disabled, onSave }: Props) {
  const parsed = parsePersonnelAbsenceManager(managerId);
  const [members, setMembers] = useState<DirectoryMemberOption[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<DirectoryMemberOption | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const res = await fetch("/api/channels/users/list", { cache: "no-store" });
        const data = (await res.json().catch(() => null)) as
          | Array<{ id?: string; name?: string; email?: string }>
          | { error?: string }
          | null;
        if (!res.ok || !Array.isArray(data)) {
          throw new Error(
            data && !Array.isArray(data) && data.error ? data.error : "Annuaire indisponible",
          );
        }
        if (cancelled) return;
        const mapped: DirectoryMemberOption[] = data
          .filter((u) => u.email)
          .map((u) => {
            const name = String(u.name || "").trim();
            const parts = name.split(/\s+/);
            return {
              externalUserId: String(u.id || u.email),
              email: String(u.email).trim().toLowerCase(),
              firstName: parts[0] || "",
              lastName: parts.slice(1).join(" ") || "",
              displayName: name || String(u.email),
            };
          });
        setMembers(mapped);
        if (parsed) {
          const hit =
            mapped.find(
              (m) =>
                (parsed.userId && m.externalUserId === parsed.userId) ||
                (parsed.email && m.email === parsed.email),
            ) || null;
          setSelected(
            hit ||
              (parsed.email
                ? {
                    externalUserId: parsed.userId || parsed.email,
                    email: parsed.email,
                    firstName: "",
                    lastName: "",
                    displayName: parsed.label || parsed.email,
                  }
                : null),
          );
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Erreur annuaire");
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
    // Intentional: hydrate once from current managerId
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const persist = async (member: DirectoryMemberOption | null) => {
    setBusy(true);
    setError(null);
    try {
      const next = serializePersonnelAbsenceManager(
        member
          ? {
              email: member.email,
              userId: member.externalUserId,
              label: directoryMemberLabel(member),
            }
          : null,
      );
      await onSave(next);
      setSelected(member);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Enregistrement impossible");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="rounded-xl border border-slate-200 bg-slate-50/80 p-4 space-y-3">
      <div>
        <h4 className="text-xs font-black uppercase tracking-widest text-slate-500">
          Absences — qui valide ?
        </h4>
        <p className="text-xs text-slate-500 mt-1">
          Par défaut, les absences du personnel OGEC vont à la direction du lycée. Choisissez ici une
          autre personne (ex. directrice de l’école) pour cette fiche uniquement. Après validation, le
          dossier reste traité par la compta / RH.
        </p>
      </div>
      {error ? <p className="text-sm text-rose-600 font-medium">{error}</p> : null}
      <DirectoryPersonSelect
        members={members}
        loading={loading || busy || disabled}
        selectedId={selected?.externalUserId}
        selectedEmail={selected?.email}
        onChange={(m) => {
          void persist(m);
        }}
      />
      {!selected ? (
        <p className="text-[11px] text-slate-500">
          Aucun rattachement → direction du lycée (ou liste globale si configurée).
        </p>
      ) : null}
    </section>
  );
}
