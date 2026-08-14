"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { SettingsSection } from "@/app/components/settings/SettingsChrome";
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
    <SettingsSection
      icon="🚪"
      title="Administrateurs du module réservation de salles"
      description="Sélectionnez les personnes dans Clerk. Elles auront le mode administrateur dans l’espace réservation de salles et pourront gérer le paramétrage (salles, matières, couleurs)."
    >
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
    </SettingsSection>
  );
}
