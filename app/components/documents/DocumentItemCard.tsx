"use client";

import { useRef, useState } from "react";
import { DocumentFileIcon, DocumentFolderIcon } from "@/app/components/documents/DocumentSystemIcons";
import {
  documentAccent,
  documentDisplayName,
  documentMetaLine,
  INCOMING_SHARED_FILES_FOLDER,
  isMacContextClick,
  type DocumentItem,
} from "@/app/lib/documents-page-model";
import DocumentContextMenu from "./DocumentContextMenu";

export default function DocumentItemCard({
  item,
  busy,
  onOpen,
  onMove,
  onDelete,
  onShare,
  onLeave,
  folderVariant,
  onShowAccess,
}: {
  item: DocumentItem;
  busy?: boolean;
  onOpen: () => void;
  onMove?: () => void;
  onDelete?: () => void;
  onShare?: () => void;
  onLeave?: () => void;
  folderVariant?: "shared-incoming" | "default";
  onShowAccess?: () => void;
}) {
  const metaLine = documentMetaLine(item);
  const hoverBorder = documentAccent(item, folderVariant);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  /** Ignore le `click` qui suit un Ctrl+clic / clic droit (comportement Mac). */
  const suppressOpenClickRef = useRef(false);

  return (
    <>
      <div
        role="button"
        tabIndex={0}
        onClick={(e) => {
          if (suppressOpenClickRef.current) {
            suppressOpenClickRef.current = false;
            return;
          }
          if (isMacContextClick(e)) return;
          onOpen();
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          suppressOpenClickRef.current = true;
          setContextMenu({ x: e.clientX, y: e.clientY });
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onOpen();
          }
        }}
        title={documentDisplayName(item)}
        className={`group relative flex flex-col items-center p-2.5 rounded-2xl border-2 border-transparent bg-transparent transition-all cursor-pointer w-full min-w-0 hover:bg-white/70 hover:shadow-sm ${hoverBorder}`}
      >
        <div className="mb-1.5 shrink-0">
          {item.type === "folder" ? (
            <DocumentFolderIcon variant={folderVariant} />
          ) : (
            <DocumentFileIcon ext={item.ext} />
          )}
        </div>
        <span className="text-center text-[10px] font-medium text-gray-700 w-full leading-snug break-words px-0.5 line-clamp-2">
          {item.name}
        </span>
        <span className="mt-0.5 text-[9px] font-medium text-gray-500 text-center leading-tight">
          {metaLine}
        </span>
        {item.isVirtual && item.name !== INCOMING_SHARED_FILES_FOLDER ? (
          <span className="text-[8px] text-indigo-500 font-medium mt-0.5 text-center leading-tight">
            Partagé
          </span>
        ) : null}
        {busy ? <span className="mt-0.5 text-[9px] text-blue-600 font-bold">…</span> : null}
      </div>

      {contextMenu ? (
        <DocumentContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
          onOpen={onOpen}
          onShare={onShare}
          onMove={onMove}
          onDelete={onDelete}
          onLeave={onLeave}
          onShowAccess={onShowAccess}
        />
      ) : null}
    </>
  );
}
