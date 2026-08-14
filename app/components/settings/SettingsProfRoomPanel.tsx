"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ProfRoomAdminPicker, { type ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

export default function SettingsProfRoomPanel({
  clerkMembers,
  profRoomAdminIds,
  setProfRoomAdminIds,
  membersLoading,
  saving,
  saveSection,
}: {
  clerkMembers: ClerkMemberOption[];
  profRoomAdminIds: string[];
  setProfRoomAdminIds: Dispatch<SetStateAction<string[]>>;
  membersLoading: boolean;
  saving: boolean;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  return (
    <div className="bg-white rounded-2xl border p-6 space-y-4">
      <h2 className="text-lg font-bold text-slate-900">Administrateurs du module réservation de salles</h2>
      <p className="text-sm text-slate-500">
        Sélectionnez les personnes dans Clerk. Elles auront le mode administrateur dans l&apos;espace réservation
        de salles et pourront gérer le paramétrage (salles, matières, couleurs).
      </p>
      <ProfRoomAdminPicker
        members={clerkMembers}
        selectedIds={profRoomAdminIds}
        onChange={setProfRoomAdminIds}
        loading={membersLoading}
      />
      <ModuleButton
        variant="primary"
        disabled={saving || membersLoading}
        onClick={() =>
          saveSection("prof-room", {
            adminClerkUserIds: profRoomAdminIds,
          })
        }
      >
        Enregistrer les administrateurs salles
      </ModuleButton>
    </div>
  );
}
