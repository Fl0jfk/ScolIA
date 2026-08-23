"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { useEffect, useMemo, useState } from "react";
import TravelsTeacherPicker from "@/app/components/travels/TravelsTeacherPicker";
import type { DirectoryAssigneeOption } from "@/app/components/domain-planning/DomainAssigneePicker";
import { userHasAdministratifRoleFromMetadata } from "@/app/lib/travels-roles";

export type TravelsOwnerFields = {
  ownerId: string;
  ownerName: string;
  ownerEmail: string;
};

type Props = {
  disabled?: boolean;
  onOwnerFieldsChange: (fields: TravelsOwnerFields | null) => void;
  onPendingChange?: (pending: boolean) => void;
};

export default function TravelsOwnerAssignSection({
  disabled,
  onOwnerFieldsChange,
  onPendingChange,
}: Props) {
  const { user, isLoaded } = useSessionUser();
  const isAdministratif = userHasAdministratifRoleFromMetadata(user?.publicMetadata as Record<string, unknown>);
  const [assignForOther, setAssignForOther] = useState(false);
  const [users, setUsers] = useState<DirectoryAssigneeOption[]>([]);
  const [loadingUsers, setLoadingUsers] = useState(false);
  const [selectedId, setSelectedId] = useState("");

  const selected = useMemo(
    () => users.find((u) => u.externalUserId === selectedId) ?? null,
    [users, selectedId],
  );

  useEffect(() => {
    if (!isLoaded || !isAdministratif || disabled) return;
    let cancelled = false;
    setLoadingUsers(true);
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
  }, [isLoaded, isAdministratif, disabled]);

  useEffect(() => {
    onPendingChange?.(Boolean(!disabled && assignForOther && !selectedId && !loadingUsers));
  }, [disabled, assignForOther, selectedId, loadingUsers, onPendingChange]);

  useEffect(() => {
    if (disabled || !assignForOther || !selected) {
      onOwnerFieldsChange(null);
      return;
    }
    const ownerName =
      selected.displayName ||
      `${selected.firstName ?? ""} ${selected.lastName ?? ""}`.trim() ||
      selected.email;
    onOwnerFieldsChange({
      ownerId: selected.externalUserId,
      ownerName,
      ownerEmail: selected.email,
    });
  }, [disabled, assignForOther, selected, onOwnerFieldsChange]);

  if (!isLoaded || !isAdministratif || disabled) return null;

  return (
    <div className="md:col-span-2 flex flex-col gap-3 p-4 bg-amber-50/80 border border-amber-200 rounded-2xl">
      <label className="flex items-start gap-3 cursor-pointer">
        <input
          type="checkbox"
          className="mt-1 w-5 h-5 accent-amber-600"
          checked={assignForOther}
          onChange={(e) => {
            setAssignForOther(e.target.checked);
            if (!e.target.checked) setSelectedId("");
          }}
        />
        <span>
          <span className="block text-sm font-bold text-slate-900">
            Créer ce dossier au nom d&apos;un enseignant
          </span>
          <span className="block text-xs text-slate-600 mt-0.5">
            Le professeur choisi sera responsable du dossier et recevra les relances par e-mail.
          </span>
        </span>
      </label>
      {assignForOther && (
        <div className="pt-2 border-t border-amber-200/80">
          <label className="block text-xs font-bold uppercase tracking-widest text-amber-900/70 mb-2">
            Enseignant responsable
          </label>
          <TravelsTeacherPicker
            users={users}
            value={selectedId}
            loading={loadingUsers}
            onChange={(u) => setSelectedId(u?.externalUserId ?? "")}
          />
          {assignForOther && !selectedId && !loadingUsers && (
            <p className="mt-2 text-xs font-semibold text-amber-800">
              Sélectionnez un utilisateur avant d&apos;enregistrer.
            </p>
          )}
        </div>
      )}
    </div>
  );
}

export function resolveTripOwnerFields(
  user: {
    id?: string | null;
    fullName?: string | null;
    primaryEmailAddress?: { emailAddress?: string } | null;
  } | null | undefined,
  override: TravelsOwnerFields | null,
): TravelsOwnerFields {
  if (override?.ownerId) return override;
  return {
    ownerId: user?.id || "",
    ownerName: user?.fullName || "Enseignant",
    ownerEmail: user?.primaryEmailAddress?.emailAddress || "",
  };
}
