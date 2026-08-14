"use client";

import {
  isMacContextClick,
  type DocumentScope,
  type ShareInfo,
} from "@/app/lib/documents-page-model";

export default function DocumentsSidebar({
  scope,
  shareId,
  isInIncomingSharedFolder,
  incomingFileCount,
  shares,
  onGoToPersonalRoot,
  onOpenIncomingSharedFiles,
  onOpenShare,
  onShareContextMenu,
}: {
  scope: DocumentScope;
  shareId: string | null;
  isInIncomingSharedFolder: boolean;
  incomingFileCount: number;
  shares: ShareInfo[];
  onGoToPersonalRoot: () => void;
  onOpenIncomingSharedFiles: () => void;
  onOpenShare: (share: ShareInfo) => void;
  onShareContextMenu: (share: ShareInfo, x: number, y: number) => void;
}) {
  return (
    <aside
      data-tour="documents-scope"
      className="md:w-56 shrink-0 bg-white border border-gray-200 rounded-2xl p-3 flex flex-col max-h-[min(720px,calc(100vh-10rem))] md:min-h-[480px]"
    >
      <button
        type="button"
        onClick={onGoToPersonalRoot}
        className={`w-full text-left px-3 py-2 rounded-xl text-sm font-semibold mb-1 ${
          scope === "personal" && !isInIncomingSharedFolder
            ? "bg-blue-50 text-blue-700"
            : "hover:bg-gray-50 text-gray-700"
        }`}
      >
        Mon cloud
      </button>
      <button
        type="button"
        onClick={onOpenIncomingSharedFiles}
        className={`w-full text-left px-3 py-2 rounded-xl text-sm mb-1 flex items-center justify-between gap-2 ${
          isInIncomingSharedFolder
            ? "bg-indigo-50 text-indigo-700 font-semibold"
            : "hover:bg-gray-50 text-gray-700"
        }`}
      >
        <span className="truncate">Fichiers partagés</span>
        {incomingFileCount > 0 && (
          <span className="shrink-0 text-[10px] font-bold bg-indigo-100 text-indigo-700 px-1.5 py-0.5 rounded-full">
            {incomingFileCount}
          </span>
        )}
      </button>
      <p className="px-3 pt-2 pb-1 text-[10px] font-bold uppercase tracking-wide text-gray-400 shrink-0">
        Dossiers partagés
      </p>
      <div className="flex-1 min-h-0 overflow-y-auto -mx-1 px-1">
        {shares.length === 0 ? (
          <p className="px-3 text-xs text-gray-400 italic">Aucun dossier partagé</p>
        ) : (
          shares.map((share) => (
            <button
              key={share.id}
              type="button"
              onClick={(e) => {
                if (isMacContextClick(e)) return;
                onOpenShare(share);
              }}
              onContextMenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onShareContextMenu(share, e.clientX, e.clientY);
              }}
              className={`w-full text-left px-3 py-2 rounded-xl text-sm mb-0.5 truncate ${
                scope === "shared" && shareId === share.id
                  ? "bg-blue-50 text-blue-700 font-semibold"
                  : "hover:bg-gray-50 text-gray-700"
              }`}
              title={`${share.name} — clic droit pour les options`}
            >
              {share.isOwner ? "👑 " : "👥 "}
              {share.name}
            </button>
          ))
        )}
      </div>
    </aside>
  );
}
