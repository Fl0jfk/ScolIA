"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type DragEvent } from "react";
import { useUser } from "@clerk/nextjs";
import AccessModal from "@/app/components/documents/AccessModal";
import DocumentItemCard from "@/app/components/documents/DocumentItemCard";
import DocumentModal from "@/app/components/documents/DocumentModal";
import DocumentsGrid from "@/app/components/documents/DocumentsGrid";
import DocumentsSidebar from "@/app/components/documents/DocumentsSidebar";
import DocumentsToolbar from "@/app/components/documents/DocumentsToolbar";
import MoveDestBreadcrumb from "@/app/components/documents/MoveDestBreadcrumb";
import PeerPicker from "@/app/components/documents/PeerPicker";
import ShareSidebarContextMenu from "@/app/components/documents/ShareSidebarContextMenu";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import {
  buildAccessPeople,
  collectDroppedFiles,
  documentDisplayName,
  fileShareIdFromPath,
  INCOMING_SHARED_FILES_FOLDER,
  isVirtualFileSharePath,
  peerFullName,
  peerRoleLabels,
  resolveItemAccess,
  type AccessPerson,
  type DocumentItem,
  type DocumentScope,
  type DropFile,
  type FileShareMetaBrief,
  type Peer,
  type QuotaInfo,
  type ShareInfo,
} from "@/app/lib/documents-page-model";

export default function DocumentsPage() {
  const { isLoaded, user } = useUser();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [scope, setScope] = useState<DocumentScope>("personal");
  const [shareId, setShareId] = useState<string | null>(null);
  const [currentPath, setCurrentPath] = useState("");
  const [items, setItems] = useState<DocumentItem[]>([]);
  const [shares, setShares] = useState<ShareInfo[]>([]);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [quota, setQuota] = useState<QuotaInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [uploading, setUploading] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [openingFile, setOpeningFile] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [showNewFolder, setShowNewFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState("");
  const [showNewShare, setShowNewShare] = useState(false);
  const [newShareName, setNewShareName] = useState("");
  const [newShareMembers, setNewShareMembers] = useState<string[]>([]);
  const [showShareManage, setShowShareManage] = useState(false);
  const [manageMembers, setManageMembers] = useState<string[]>([]);
  const [peerSearch, setPeerSearch] = useState("");
  const [moveItem, setMoveItem] = useState<DocumentItem | null>(null);
  const [moveDestPath, setMoveDestPath] = useState("");
  const [moveDestFolders, setMoveDestFolders] = useState<DocumentItem[]>([]);
  const [moveDestLoading, setMoveDestLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [showDeleteFolder, setShowDeleteFolder] = useState(false);
  const [deleteFolderConfirm, setDeleteFolderConfirm] = useState("");
  const [showLeaveShare, setShowLeaveShare] = useState(false);
  const [shareFileItem, setShareFileItem] = useState<DocumentItem | null>(null);
  const [shareFileMembers, setShareFileMembers] = useState<string[]>([]);
  const [incomingFileCount, setIncomingFileCount] = useState(0);
  const [incomingFileShares, setIncomingFileShares] = useState<FileShareMetaBrief[]>([]);
  const [outgoingFileShares, setOutgoingFileShares] = useState<FileShareMetaBrief[]>([]);
  const [accessModal, setAccessModal] = useState<{ title: string; people: AccessPerson[] } | null>(null);
  const [sidebarShareMenu, setSidebarShareMenu] = useState<{
    share: ShareInfo;
    x: number;
    y: number;
  } | null>(null);

  useEffect(() => {
    const pathFromUrl = new URLSearchParams(window.location.search).get("path");
    if (pathFromUrl) {
      setCurrentPath(pathFromUrl.endsWith("/") ? pathFromUrl : `${pathFromUrl}/`);
    }
  }, []);

  const activeShare = useMemo(
    () => shares.find((s) => s.id === shareId) ?? null,
    [shares, shareId],
  );

  const openAccessModal = useCallback(
    (title: string, ownerId: string, memberIds: string[]) => {
      if (!user?.id) return;
      setAccessModal({
        title,
        people: buildAccessPeople(ownerId, memberIds, peers, user.id),
      });
    },
    [peers, user?.id],
  );

  const openShareAccess = useCallback(
    (share: ShareInfo) => {
      openAccessModal(share.name, share.ownerId, share.memberIds);
    },
    [openAccessModal],
  );

  const pathSegments = useMemo(() => {
    if (!currentPath) return [];
    return currentPath.replace(/\/$/, "").split("/").filter(Boolean);
  }, [currentPath]);

  const refreshQuota = useCallback(async () => {
    try {
      const res = await fetch("/api/documents/quota", { cache: "no-store" });
      if (res.ok) setQuota(await res.json());
    } catch {
      /* ignore */
    }
  }, []);

  const refreshShares = useCallback(async () => {
    try {
      const res = await fetch("/api/documents/shares", { cache: "no-store" });
      if (res.ok) {
        const data = await res.json();
        setShares(data.shares ?? []);
      }
    } catch {
      /* ignore */
    }
  }, []);

  const fetchDocuments = useCallback(async () => {
    if (scope === "shared" && !shareId) {
      setItems([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams({ scope, path: currentPath });
      if (shareId) params.set("shareId", shareId);
      const res = await fetch(`/api/documents/browse?${params}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) {
        setItems([]);
        setError(data.error || "Impossible de charger le dossier.");
        return;
      }
      setItems(data.items ?? []);
    } catch {
      setItems([]);
      setError("Erreur de connexion.");
    } finally {
      setLoading(false);
    }
  }, [scope, shareId, currentPath]);

  const refreshFileShares = useCallback(async () => {
    try {
      const res = await fetch("/api/documents/file-shares", { cache: "no-store" });
      if (res.ok) {
        const data = await res.json();
        const incoming = (data.incoming ?? data.shares ?? []) as FileShareMetaBrief[];
        const outgoing = (data.outgoing ?? []) as FileShareMetaBrief[];
        setIncomingFileShares(incoming);
        setOutgoingFileShares(outgoing);
        setIncomingFileCount(incoming.length);
      }
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    if (!isLoaded) return;
    refreshShares();
    refreshQuota();
    refreshFileShares();
    fetch("/api/documents/peers", { cache: "no-store" })
      .then((r) => r.json())
      .then((d) => setPeers(d.peers ?? []))
      .catch(() => {});
  }, [isLoaded, refreshShares, refreshQuota, refreshFileShares]);

  useEffect(() => {
    if (isLoaded) fetchDocuments();
  }, [isLoaded, fetchDocuments]);

  const goToPersonalRoot = () => {
    setScope("personal");
    setShareId(null);
    setCurrentPath("");
  };

  const openIncomingSharedFiles = () => {
    setScope("personal");
    setShareId(null);
    setCurrentPath(`${INCOMING_SHARED_FILES_FOLDER}/`);
  };

  const isInIncomingSharedFolder =
    scope === "personal" &&
    (currentPath === `${INCOMING_SHARED_FILES_FOLDER}/` ||
      currentPath === INCOMING_SHARED_FILES_FOLDER);

  const openShare = (share: ShareInfo) => {
    setScope("shared");
    setShareId(share.id);
    setCurrentPath("");
  };

  const enterFolder = (relPath: string) => {
    setCurrentPath(relPath.endsWith("/") ? relPath : `${relPath}/`);
  };

  const enterFolderInMovePicker = (relPath: string) => {
    setMoveDestPath(relPath.endsWith("/") ? relPath : `${relPath}/`);
  };

  const navigateToSegment = (index: number) => {
    if (index < 0) {
      if (scope === "shared" && shareId) {
        setCurrentPath("");
        return;
      }
      goToPersonalRoot();
      return;
    }
    const parts = pathSegments.slice(0, index + 1);
    setCurrentPath(parts.length ? `${parts.join("/")}/` : "");
  };

  const handleOpenFile = async (relPath: string) => {
    setOpeningFile(relPath);
    try {
      const params = new URLSearchParams({ scope, path: relPath });
      if (shareId) params.set("shareId", shareId);
      const res = await fetch(`/api/documents/get-url?${params}`);
      const data = await res.json();
      if (data.url) window.open(data.url, "_blank", "noopener,noreferrer");
      else setError(data.error || "Ouverture impossible.");
    } catch {
      setError("Erreur lors de l'ouverture du fichier.");
    } finally {
      setOpeningFile(null);
    }
  };

  const uploadFiles = async (files: DropFile[]) => {
    if (files.length === 0) return;
    setUploading(true);
    setError(null);
    try {
      const formData = new FormData();
      formData.append("scope", scope);
      formData.append("path", currentPath);
      if (shareId) formData.append("shareId", shareId);
      for (const { file, relPath } of files) {
        formData.append("file", file);
        formData.append(`relPath:${file.name}`, relPath);
      }
      const res = await fetch("/api/documents/upload", { method: "POST", body: formData });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Échec de l'envoi.");
        return;
      }
      await fetchDocuments();
      await refreshQuota();
    } catch {
      setError("Erreur lors de l'envoi des fichiers.");
    } finally {
      setUploading(false);
    }
  };

  const loadMoveDestFolders = useCallback(
    async (path: string) => {
      if (scope === "shared" && !shareId) return;
      setMoveDestLoading(true);
      try {
        const params = new URLSearchParams({ scope, path });
        if (shareId) params.set("shareId", shareId);
        const res = await fetch(`/api/documents/browse?${params}`, { cache: "no-store" });
        const data = await res.json();
        setMoveDestFolders((data.items ?? []).filter((i: DocumentItem) => i.type === "folder"));
      } catch {
        setMoveDestFolders([]);
      } finally {
        setMoveDestLoading(false);
      }
    },
    [scope, shareId],
  );

  useEffect(() => {
    if (moveItem) {
      setMoveDestPath("");
      loadMoveDestFolders("");
    }
  }, [moveItem, loadMoveDestFolders]);

  useEffect(() => {
    if (moveItem) loadMoveDestFolders(moveDestPath);
  }, [moveDestPath, moveItem, loadMoveDestFolders]);

  const parentPathOf = (relPath: string, isFolder: boolean) => {
    const parts = relPath.replace(/\/$/, "").split("/").filter(Boolean);
    if (!isFolder) parts.pop();
    else parts.pop();
    return parts.length ? `${parts.join("/")}/` : "";
  };

  const handleDeleteItem = async (item: DocumentItem) => {
    const label = item.type === "folder" ? "ce dossier et son contenu" : "ce fichier";
    if (!window.confirm(`Supprimer ${label} ?`)) return;
    setActionLoading(item.relPath);
    setError(null);
    try {
      const params = new URLSearchParams({
        scope,
        path: item.relPath,
        itemType: item.type,
      });
      if (shareId) params.set("shareId", shareId);
      const res = await fetch(`/api/documents/delete?${params}`, { method: "DELETE" });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Suppression impossible.");
        return;
      }
      await fetchDocuments();
      await refreshQuota();
    } catch {
      setError("Erreur lors de la suppression.");
    } finally {
      setActionLoading(null);
    }
  };

  const handleConfirmMove = async () => {
    if (!moveItem) return;
    setActionLoading(moveItem.relPath);
    setError(null);
    try {
      const res = await fetch("/api/documents/move", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          scope,
          shareId,
          sourcePath: moveItem.relPath,
          destParentPath: moveDestPath,
          itemType: moveItem.type,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Déplacement impossible.");
        return;
      }
      setMoveItem(null);
      await fetchDocuments();
    } catch {
      setError("Erreur lors du déplacement.");
    } finally {
      setActionLoading(null);
    }
  };

  const handleDrop = async (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    if (uploading) return;
    const files = await collectDroppedFiles(e.dataTransfer);
    await uploadFiles(files);
  };

  const handleCreateFolder = async () => {
    const name = newFolderName.trim();
    if (!name) return;
    setError(null);
    const res = await fetch("/api/documents/mkdir", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ scope, shareId, path: currentPath, name }),
    });
    const data = await res.json();
    if (!res.ok) {
      setError(data.error || "Création impossible.");
      return;
    }
    setNewFolderName("");
    setShowNewFolder(false);
    await fetchDocuments();
  };

  const handleCreateShare = async () => {
    const name = newShareName.trim();
    if (!name) return;
    setError(null);
    const res = await fetch("/api/documents/shares", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, memberIds: newShareMembers }),
    });
    const data = await res.json();
    if (!res.ok) {
      setError(data.error || "Création impossible.");
      return;
    }
    setShowNewShare(false);
    setNewShareName("");
    setNewShareMembers([]);
    await refreshShares();
    if (data.share) openShare({ ...data.share, isOwner: true });
  };

  const handleUpdateShareMembers = async () => {
    if (!activeShare) return;
    setError(null);
    const res = await fetch("/api/documents/shares", {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ shareId: activeShare.id, memberIds: manageMembers }),
    });
    const data = await res.json();
    if (!res.ok) {
      setError(data.error || "Mise à jour impossible.");
      return;
    }
    setShowShareManage(false);
    await refreshShares();
  };

  const filteredPeers = useMemo(() => {
    const q = peerSearch.trim().toLowerCase();
    if (!q) return peers;
    return peers.filter(
      (p) =>
        peerFullName(p).toLowerCase().includes(q) ||
        peerRoleLabels(p).toLowerCase().includes(q) ||
        p.email.toLowerCase().includes(q) ||
        p.roles.some((r) => r.toLowerCase().includes(q)),
    );
  }, [peers, peerSearch]);

  const toggleMember = (id: string, list: string[], setter: (v: string[]) => void) => {
    setter(list.includes(id) ? list.filter((x) => x !== id) : [...list, id]);
  };

  const handleDeleteCurrentFolder = async () => {
    if (deleteFolderConfirm.trim().toLowerCase() !== "supprimer") {
      setError('Tapez « supprimer » pour confirmer.');
      return;
    }
    setActionLoading("delete-folder");
    setError(null);
    try {
      const res = await fetch("/api/documents/delete-folder", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          scope,
          shareId,
          folderPath: currentFolderRel,
          confirm: deleteFolderConfirm,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Suppression impossible.");
        return;
      }
      setShowDeleteFolder(false);
      setDeleteFolderConfirm("");
      if (data.shareDeleted) {
        goToPersonalRoot();
        await refreshShares();
      } else if (pathSegments.length > 1) {
        navigateToSegment(pathSegments.length - 2);
      } else {
        setCurrentPath("");
      }
      await fetchDocuments();
      await refreshQuota();
    } catch {
      setError("Erreur lors de la suppression du dossier.");
    } finally {
      setActionLoading(null);
    }
  };

  const handleShareFile = async () => {
    if (!shareFileItem) return;
    setError(null);
    const res = await fetch("/api/documents/file-shares", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        sourcePath: shareFileItem.relPath,
        memberIds: shareFileMembers,
      }),
    });
    const data = await res.json();
    if (!res.ok) {
      setError(data.error || "Partage impossible.");
      return;
    }
    setShareFileItem(null);
    setShareFileMembers([]);
    await refreshFileShares();
  };

  const handleLeaveFileShare = async (item: DocumentItem) => {
    const id = fileShareIdFromPath(item.relPath);
    if (!id) return;
    setActionLoading(item.relPath);
    setError(null);
    try {
      const res = await fetch("/api/documents/file-shares/leave", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fileShareId: id }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Impossible de retirer le fichier.");
        return;
      }
      await fetchDocuments();
      await refreshFileShares();
    } catch {
      setError("Erreur lors du retrait du fichier partagé.");
    } finally {
      setActionLoading(null);
    }
  };

  const handleLeaveShare = async () => {
    if (!shareId) return;
    setActionLoading("leave-share");
    setError(null);
    try {
      const res = await fetch("/api/documents/shares/leave", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ shareId }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error || "Impossible de quitter le dossier.");
        return;
      }
      setShowLeaveShare(false);
      goToPersonalRoot();
      await refreshShares();
    } catch {
      setError("Erreur lors de la sortie du dossier partagé.");
    } finally {
      setActionLoading(null);
    }
  };

  if (!isLoaded) return null;

  const isSharePicker = scope === "shared" && !shareId;
  const rootLabel = isInIncomingSharedFolder
    ? INCOMING_SHARED_FILES_FOLDER
    : scope === "personal"
      ? "Mon cloud"
      : activeShare?.name ?? "Dossier partagé";
  const isShareOwner = Boolean(activeShare?.isOwner);
  const canDeleteCurrentFolder =
    !isSharePicker &&
    !isInIncomingSharedFolder &&
    ((scope === "personal" && currentPath !== "") ||
      (scope === "shared" && isShareOwner && Boolean(shareId)));
  const currentFolderRel = currentPath.replace(/\/$/, "");
  const deleteFolderLabel =
    scope === "shared" && !currentFolderRel
      ? "Supprimer le dossier partagé"
      : "Supprimer ce dossier";
  const deleteFolderTargetName =
    scope === "shared" && !currentFolderRel
      ? activeShare?.name ?? "dossier partagé"
      : pathSegments[pathSegments.length - 1] ?? "ce dossier";
  const isDeletingEntireShare = scope === "shared" && !currentFolderRel;

  return (
    <ModulePageShell className="flex flex-col gap-5">
      <div data-tour="documents-intro">
        <ModulePageHeader
          eyebrow="Services"
          title="Documents"
          description="Cloud personnel et dossiers partagés avec le personnel Clerk."
          actions={<ReplayModuleTourButton moduleId="documents" />}
        />
      </div>

      <section className="bg-white border border-gray-200 shadow-sm p-5 rounded-2xl flex flex-col lg:flex-row gap-4 lg:items-center lg:justify-between">
        <div className="min-w-0 flex-1">
          {quota && (
            <div data-tour="documents-quota">
              <div className="flex justify-between text-xs text-gray-500 mb-1">
                <span>Espace utilisé</span>
                <span>
                  {quota.usedLabel} / {quota.quotaLabel}
                </span>
              </div>
              <div className="h-2 rounded-full bg-gray-100 overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all ${quota.percent > 90 ? "bg-red-500" : "bg-blue-600"}`}
                  style={{ width: `${quota.percent}%` }}
                />
              </div>
            </div>
          )}
        </div>
        <DocumentsToolbar
          fileInputRef={fileInputRef}
          uploading={uploading}
          uploadDisabled={isSharePicker || isInIncomingSharedFolder}
          canDeleteCurrentFolder={canDeleteCurrentFolder}
          deleteFolderLabel={deleteFolderLabel}
          activeShare={activeShare}
          shareId={shareId}
          canLeaveShare={scope === "shared" && Boolean(shareId) && !isShareOwner}
          onNewFolder={() => setShowNewFolder(true)}
          onPickFiles={() => fileInputRef.current?.click()}
          onNewShare={() => setShowNewShare(true)}
          onShowAccess={() => {
            if (activeShare) openShareAccess(activeShare);
          }}
          onManageAccess={() => {
            if (!activeShare) return;
            setManageMembers(activeShare.memberIds);
            setShowShareManage(true);
          }}
          onDeleteFolder={() => {
            setDeleteFolderConfirm("");
            setShowDeleteFolder(true);
          }}
          onLeaveShare={() => setShowLeaveShare(true)}
          onFilesSelected={(files) => {
            void uploadFiles(files.map((file) => ({ file, relPath: file.name })));
          }}
        />
      </section>

      <div className="flex flex-col md:flex-row gap-4 md:items-stretch">
        <DocumentsSidebar
          scope={scope}
          shareId={shareId}
          isInIncomingSharedFolder={isInIncomingSharedFolder}
          incomingFileCount={incomingFileCount}
          shares={shares}
          onGoToPersonalRoot={goToPersonalRoot}
          onOpenIncomingSharedFiles={openIncomingSharedFiles}
          onOpenShare={openShare}
          onShareContextMenu={(share, x, y) => setSidebarShareMenu({ share, x, y })}
        />

        <DocumentsGrid
          rootLabel={rootLabel}
          pathSegments={pathSegments}
          error={error}
          isSharePicker={isSharePicker}
          dragActive={dragActive}
          uploading={uploading}
          loading={loading}
          hasItems={items.length > 0}
          onNavigateSegment={navigateToSegment}
          onDrop={handleDrop}
          onDragActiveChange={setDragActive}
        >
          {items.map((item) => {
            const virtualFile = isVirtualFileSharePath(item.relPath);
            const canShareFile =
              scope === "personal" &&
              !isInIncomingSharedFolder &&
              item.type === "file" &&
              !virtualFile;
            const access = resolveItemAccess(item, {
              scope,
              activeShare,
              isInIncomingSharedFolder,
              incomingFileShares,
              outgoingFileShares,
            });
            const accessPeople =
              access && user?.id
                ? buildAccessPeople(access.ownerId, access.memberIds, peers, user.id)
                : undefined;
            const accessTitle = documentDisplayName(item);
            return (
              <DocumentItemCard
                key={item.relPath}
                item={item}
                busy={actionLoading === item.relPath || openingFile === item.relPath}
                onShowAccess={
                  accessPeople
                    ? () => openAccessModal(accessTitle, access!.ownerId, access!.memberIds)
                    : undefined
                }
                onOpen={() => {
                  if (item.type === "folder") {
                    enterFolder(
                      item.relPath.endsWith("/") ? item.relPath : `${item.relPath}/`,
                    );
                  } else handleOpenFile(item.relPath);
                }}
                onMove={virtualFile ? undefined : () => setMoveItem(item)}
                onDelete={virtualFile ? undefined : () => handleDeleteItem(item)}
                onShare={canShareFile ? () => setShareFileItem(item) : undefined}
                onLeave={virtualFile ? () => handleLeaveFileShare(item) : undefined}
                folderVariant={
                  item.name === INCOMING_SHARED_FILES_FOLDER ? "shared-incoming" : "default"
                }
              />
            );
          })}
        </DocumentsGrid>
      </div>

      {showDeleteFolder && (
        <DocumentModal title={deleteFolderLabel} onClose={() => setShowDeleteFolder(false)}>
          <p className="text-sm text-gray-600 mb-3">
            {isDeletingEntireShare ? (
              <>
                Cette action supprime définitivement le dossier partagé{" "}
                <strong>{deleteFolderTargetName}</strong>, tout son contenu et l&apos;accès pour tous
                les membres. Elle est irréversible.
              </>
            ) : (
              <>
                Cette action supprime définitivement <strong>{deleteFolderTargetName}</strong> et tout son
                contenu. Elle est irréversible.
              </>
            )}
          </p>
          <p className="text-sm text-gray-500 mb-2">
            Tapez <strong>supprimer</strong> pour confirmer :
          </p>
          <input
            autoFocus
            value={deleteFolderConfirm}
            onChange={(e) => setDeleteFolderConfirm(e.target.value)}
            placeholder="supprimer"
            className="w-full border border-gray-300 rounded-xl px-3 py-2 text-sm mb-4"
          />
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setShowDeleteFolder(false)}
              className="px-4 py-2 text-sm rounded-xl border"
            >
              Annuler
            </button>
            <button
              type="button"
              onClick={handleDeleteCurrentFolder}
              disabled={
                deleteFolderConfirm.trim().toLowerCase() !== "supprimer" ||
                actionLoading === "delete-folder"
              }
              className="px-4 py-2 text-sm rounded-xl bg-red-600 text-white font-semibold disabled:opacity-40"
            >
              {actionLoading === "delete-folder" ? "Suppression…" : "Supprimer définitivement"}
            </button>
          </div>
        </DocumentModal>
      )}

      {showLeaveShare && activeShare && (
        <DocumentModal title="Quitter le dossier partagé" onClose={() => setShowLeaveShare(false)}>
          <p className="text-sm text-gray-600 mb-4">
            Vous ne verrez plus <strong>{activeShare.name}</strong> dans vos dossiers partagés. Le
            propriétaire pourra vous y réintégrer à tout moment via « Gérer le partage ».
          </p>
          <div className="flex justify-end gap-2">
            <button type="button" onClick={() => setShowLeaveShare(false)} className="px-4 py-2 text-sm rounded-xl border">
              Annuler
            </button>
            <button
              type="button"
              onClick={handleLeaveShare}
              disabled={actionLoading === "leave-share"}
              className="px-4 py-2 text-sm rounded-xl bg-amber-600 text-white font-semibold disabled:opacity-40"
            >
              {actionLoading === "leave-share" ? "En cours…" : "Quitter"}
            </button>
          </div>
        </DocumentModal>
      )}

      {shareFileItem && (
        <DocumentModal
          title={`Partager « ${shareFileItem.name}${shareFileItem.ext ? `.${shareFileItem.ext}` : ""} »`}
          onClose={() => {
            setShareFileItem(null);
            setShareFileMembers([]);
          }}
          wide
        >
          <p className="text-sm text-gray-500 mb-3">
            Les personnes choisies verront ce fichier dans leur dossier « Fichiers partagés ».
          </p>
          <PeerPicker
            peers={filteredPeers}
            search={peerSearch}
            onSearch={setPeerSearch}
            selected={shareFileMembers}
            onToggle={(id) => toggleMember(id, shareFileMembers, setShareFileMembers)}
          />
          <div className="flex justify-end gap-2 mt-4">
            <button
              type="button"
              onClick={() => {
                setShareFileItem(null);
                setShareFileMembers([]);
              }}
              className="px-4 py-2 text-sm rounded-xl border"
            >
              Annuler
            </button>
            <button
              type="button"
              onClick={handleShareFile}
              disabled={shareFileMembers.length === 0}
              className="px-4 py-2 text-sm rounded-xl bg-indigo-600 text-white font-semibold disabled:opacity-40"
            >
              Partager
            </button>
          </div>
        </DocumentModal>
      )}

      {showNewFolder && (
        <DocumentModal title="Nouveau dossier" onClose={() => setShowNewFolder(false)}>
          <input
            autoFocus
            value={newFolderName}
            onChange={(e) => setNewFolderName(e.target.value)}
            placeholder="Nom du dossier"
            className="w-full border border-gray-300 rounded-xl px-3 py-2 text-sm mb-4"
            onKeyDown={(e) => e.key === "Enter" && handleCreateFolder()}
          />
          <div className="flex justify-end gap-2">
            <button type="button" onClick={() => setShowNewFolder(false)} className="px-4 py-2 text-sm rounded-xl border">
              Annuler
            </button>
            <button type="button" onClick={handleCreateFolder} className="px-4 py-2 text-sm rounded-xl bg-blue-600 text-white font-semibold">
              Créer
            </button>
          </div>
        </DocumentModal>
      )}

      {showNewShare && (
        <DocumentModal title="Nouveau dossier partagé" onClose={() => setShowNewShare(false)} wide>
          <input
            autoFocus
            value={newShareName}
            onChange={(e) => setNewShareName(e.target.value)}
            placeholder="Ex. Rentrée collège 2025"
            className="w-full border border-gray-300 rounded-xl px-3 py-2 text-sm mb-3"
          />
          <PeerPicker
            peers={filteredPeers}
            search={peerSearch}
            onSearch={setPeerSearch}
            selected={newShareMembers}
            onToggle={(id) => toggleMember(id, newShareMembers, setNewShareMembers)}
          />
          <div className="flex justify-end gap-2 mt-4">
            <button type="button" onClick={() => setShowNewShare(false)} className="px-4 py-2 text-sm rounded-xl border">
              Annuler
            </button>
            <button type="button" onClick={handleCreateShare} className="px-4 py-2 text-sm rounded-xl bg-blue-600 text-white font-semibold">
              Créer et ouvrir
            </button>
          </div>
        </DocumentModal>
      )}

      {moveItem && (
        <DocumentModal title={`Déplacer « ${moveItem.name}${moveItem.ext ? `.${moveItem.ext}` : ""} »`} onClose={() => setMoveItem(null)} wide>
          <p className="text-sm text-gray-500 mb-3">Choisissez le dossier de destination (depuis la racine).</p>
          <MoveDestBreadcrumb
            rootLabel={scope === "personal" ? "Mon cloud" : activeShare?.name ?? "Dossier partagé"}
            path={moveDestPath}
            onNavigate={(path) => setMoveDestPath(path)}
          />
          <div className="border border-gray-200 rounded-xl max-h-56 overflow-y-auto divide-y mb-4">
            {moveDestLoading ? (
              <p className="p-4 text-sm text-gray-400 text-center">Chargement…</p>
            ) : moveDestFolders.length === 0 ? (
              <p className="p-4 text-sm text-gray-400 text-center italic">Aucun sous-dossier</p>
            ) : (
              moveDestFolders.map((folder) => (
                <button
                  key={folder.relPath}
                  type="button"
                  onClick={() => enterFolderInMovePicker(folder.relPath)}
                  className="w-full flex items-center gap-3 px-4 py-3 text-sm hover:bg-gray-50 text-left"
                >
                  <span className="text-xl">📁</span>
                  <span className="font-medium text-gray-800">{folder.name}</span>
                </button>
              ))
            )}
          </div>
          <div className="flex justify-end gap-2">
            <button type="button" onClick={() => setMoveItem(null)} className="px-4 py-2 text-sm rounded-xl border">
              Annuler
            </button>
            <button
              type="button"
              onClick={handleConfirmMove}
              disabled={parentPathOf(moveItem.relPath, moveItem.type === "folder") === moveDestPath}
              className="px-4 py-2 text-sm rounded-xl bg-blue-600 text-white font-semibold disabled:opacity-40"
            >
              Déplacer ici
            </button>
          </div>
        </DocumentModal>
      )}

      {showShareManage && activeShare && (
        <DocumentModal title={`Modifier les accès — ${activeShare.name}`} onClose={() => setShowShareManage(false)} wide>
          <p className="text-sm text-gray-500 mb-3">
            Vous êtes propriétaire. Ajoutez ou retirez des personnes du personnel Clerk.
          </p>
          <PeerPicker
            peers={filteredPeers}
            search={peerSearch}
            onSearch={setPeerSearch}
            selected={manageMembers}
            onToggle={(id) => toggleMember(id, manageMembers, setManageMembers)}
          />
          <div className="flex justify-end gap-2 mt-4">
            <button type="button" onClick={() => setShowShareManage(false)} className="px-4 py-2 text-sm rounded-xl border">
              Annuler
            </button>
            <button
              type="button"
              onClick={handleUpdateShareMembers}
              className="px-4 py-2 text-sm rounded-xl bg-slate-800 text-white font-semibold"
            >
              Enregistrer
            </button>
          </div>
        </DocumentModal>
      )}

      {accessModal && (
        <AccessModal
          title={accessModal.title}
          people={accessModal.people}
          onClose={() => setAccessModal(null)}
        />
      )}

      {sidebarShareMenu && (
        <ShareSidebarContextMenu
          x={sidebarShareMenu.x}
          y={sidebarShareMenu.y}
          share={sidebarShareMenu.share}
          onClose={() => setSidebarShareMenu(null)}
          onOpen={() => openShare(sidebarShareMenu.share)}
          onShowAccess={() => openShareAccess(sidebarShareMenu.share)}
          onManageAccess={
            sidebarShareMenu.share.isOwner
              ? () => {
                  setManageMembers(sidebarShareMenu.share.memberIds);
                  setShowShareManage(true);
                }
              : undefined
          }
        />
      )}
    </ModulePageShell>
  );
}
