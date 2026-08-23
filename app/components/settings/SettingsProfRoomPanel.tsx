"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { SettingsSection } from "@/app/components/settings/SettingsChrome";
import ProfRoomAdminPicker, { type DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

export default function SettingsProfRoomPanel({
  directoryMembers,
  profRoomAdminIds,
  setProfRoomAdminIds,
  membersLoading,
  saving,
  saveSection,
}: {
  directoryMembers: DirectoryMemberOption[];
  profRoomAdminIds: string[];
  setProfRoomAdminIds: Dispatch<SetStateAction<string[]>>;
  membersLoading: boolean;
  saving: boolean;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  return (
    <SettingsSection
      icon="🚪"
      title="Administrateurs du module réservation de salles"
      description="Sélectionnez les personnes dans le directory. Elles auront le mode administrateur dans l’espace réservation de salles et pourront gérer le paramétrage (salles, matières, couleurs)."
    >
      <ProfRoomAdminPicker
        members={directoryMembers}
        selectedIds={profRoomAdminIds}
        onChange={setProfRoomAdminIds}
        loading={membersLoading}
      />
      <ModuleButton
        variant="primary"
        disabled={saving || membersLoading}
        onClick={() =>
          saveSection("prof-room", {
            adminExternalUserIds: profRoomAdminIds,
          })
        }
      >
        Enregistrer les administrateurs salles
      </ModuleButton>
    </SettingsSection>
  );
}
