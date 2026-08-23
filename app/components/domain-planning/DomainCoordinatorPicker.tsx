"use client";

import ProfRoomAdminPicker, { type DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

export type { DirectoryMemberOption };

export default function DomainCoordinatorPicker({
  domainName,
  members,
  selectedIds,
  onChange,
  loading,
}: {
  domainName: string;
  members: DirectoryMemberOption[];
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  loading?: boolean;
}) {
  return (
    <ProfRoomAdminPicker
      members={members}
      selectedIds={selectedIds}
      onChange={onChange}
      loading={loading}
      footerHint={
        domainName.toLowerCase().includes("evars")
          ? `${selectedIds.length} responsable(s) EVARS. Elles valident les positionnements des intervenants.`
          : `${selectedIds.length} responsable(s) pour ${domainName || "ce domaine"}.`
      }
    />
  );
}
