"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { useUser } from "@clerk/nextjs";
import { INTRANET_ROLE_OPTIONS } from "@/app/lib/intranet-roles";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";
import { DocumentFileIcon, DocumentFolderIcon } from "@/app/components/documents/DocumentSystemIcons";

type DocumentScope = "personal" | "shared";

type DocumentItem = {
  type: "folder" | "file";
  name: string;
  relPath: string;
  ext?: string;
  size?: number;
  sharedBy?: string;
  isVirtual?: boolean;
};

const INCOMING_SHARED_FILES_FOLDER = "Fichiers partagés";
const FILE_SHARE_REL_PREFIX = "__fileshare__/";

/** Mac Ctrl+clic = menu contextuel : ne pas traiter comme un clic d'ouverture. */
function isMacContextClick(e: { ctrlKey: boolean; button: number; metaKey?: boolean }) {
  return e.ctrlKey || e.button === 2;
}

function isVirtualFileSharePath(relPath: string) {
  return relPath.startsWith(FILE_SHARE_REL_PREFIX);
}

function fileShareIdFromPath(relPath: string) {
  return relPath.slice(FILE_SHARE_REL_PREFIX.length).replace(/\/$/, "");
}

type ShareInfo = {
  id: string;
  name: string;
  ownerId: string;
  memberIds: string[];
  isOwner: boolean;
};

type Peer = {
  clerkUserId: string;
  firstName?: string;
  lastName?: string;
  email: string;
  roles: string[];
};

type FileShareMetaBrief = {
  id: string;
  ownerId: string;
  memberIds: string[];
  sourceRelPath: string;
  fileName: string;
  ext?: string;
};

type AccessPerson = {
  userId: string;
  name: string;
  detail: string;
  isOwner: boolean;
  isYou: boolean;
};

function peerFullName(p: Peer): string {
  const name = `${p.firstName ?? ""} ${p.lastName ?? ""}`.trim();
  return name || "Nom non renseigné";
}

function peerRoleLabels(p: Peer): string {
  if (!p.roles.length) return "Aucun rôle";
  return p.roles
    .map((slug) => INTRANET_ROLE_OPTIONS.find((o) => o.slug === slug)?.label ?? slug)
    .join(", ");
}

function normalizeDocPath(path: string): string {
  return path.replace(/^\/+/, "").replace(/\/+/g, "/");
}

function buildAccessPeople(
  ownerId: string,
  memberIds: string[],
  peers: Peer[],
  currentUserId: string,
): AccessPerson[] {
  const peerById = new Map(peers.map((p) => [p.clerkUserId, p]));
  const people: AccessPerson[] = [];

  const pushPerson = (userId: string, isOwner: boolean) => {
    const peer = peerById.get(userId);
    people.push({
      userId,
      name: peer ? peerFullName(peer) : "Membre du personnel",
      detail: peer ? peerRoleLabels(peer) : "Hors annuaire visible",
      isOwner,
      isYou: userId === currentUserId,
    });
  };

  pushPerson(ownerId, true);
  for (const id of memberIds) {
    if (id === ownerId) continue;
    pushPerson(id, false);
  }
  return people;
}

function resolveItemAccess(
  item: DocumentItem,
  opts: {
    scope: DocumentScope;
    activeShare: ShareInfo | null;
    isInIncomingSharedFolder: boolean;
    incomingFileShares: FileShareMetaBrief[];
    outgoingFileShares: FileShareMetaBrief[];
  },
): { ownerId: string; memberIds: string[] } | null {
  if (item.name === INCOMING_SHARED_FILES_FOLDER) return null;

  if (opts.scope === "shared" && opts.activeShare) {
    return { ownerId: opts.activeShare.ownerId, memberIds: opts.activeShare.memberIds };
  }

  if (isVirtualFileSharePath(item.relPath)) {
    const id = fileShareIdFromPath(item.relPath);
    const meta =
      opts.incomingFileShares.find((s) => s.id === id) ??
      opts.outgoingFileShares.find((s) => s.id === id);
    if (meta) return { ownerId: meta.ownerId, memberIds: meta.memberIds };
  }

  if (
    opts.scope === "personal" &&
    !opts.isInIncomingSharedFolder &&
    item.type === "file" &&
    !item.isVirtual
  ) {
    const src = normalizeDocPath(item.relPath);
    const meta = opts.outgoingFileShares.find(
      (s) => normalizeDocPath(s.sourceRelPath) === src,
    );
    if (meta) return { ownerId: meta.ownerId, memberIds: meta.memberIds };
  }

  return null;
}

type QuotaInfo = {
  used: number;
  quota: number;
  usedLabel: string;
  quotaLabel: string;
  percent: number;
};

type DropFile = { file: File; relPath: string };

function formatFileSize(bytes?: number): string | null {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return null;
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) {
    const ko = bytes / 1024;
    return `${(ko < 10 ? ko.toFixed(1) : Math.round(ko).toString()).replace(".", ",")} Ko`;
  }
  if (bytes < 1024 * 1024 * 1024) {
    const mo = bytes / (1024 * 1024);
    return `${(mo < 10 ? mo.toFixed(1) : Math.round(mo).toString()).replace(".", ",")} Mo`;
  }
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1).replace(".", ",")} Go`;
}

function documentDisplayName(item: DocumentItem): string {
  if (item.type === "file" && item.ext) return `${item.name}.${item.ext}`;
  return item.name;
}

function documentKindLabel(item: DocumentItem): string {
  if (item.name === INCOMING_SHARED_FILES_FOLDER) return "Dossier";
  if (item.type === "folder") return item.isVirtual ? "Dossier partagé" : "Dossier";
  return item.isVirtual ? "Fichier partagé" : "Fichier";
}

function documentAccent(item: DocumentItem, folderVariant?: "shared-incoming" | "default"): string {
  if (item.type === "folder") {
    if (folderVariant === "shared-incoming" || item.isVirtual) return "hover:border-indigo-300";
    return "hover:border-amber-300";
  }
  const key = (item.ext || "").toLowerCase();
  const hoverMap: Record<string, string> = {
    pdf: "hover:border-red-300",
    doc: "hover:border-blue-300",
    docx: "hover:border-blue-300",
    xls: "hover:border-emerald-300",
    xlsx: "hover:border-emerald-300",
    ppt: "hover:border-orange-300",
    pptx: "hover:border-orange-300",
    txt: "hover:border-slate-300",
    jpg: "hover:border-violet-300",
    jpeg: "hover:border-violet-300",
    png: "hover:border-violet-300",
    zip: "hover:border-amber-300",
  };
  return hoverMap[key] ?? "hover:border-slate-300";
}

function documentMetaLine(item: DocumentItem): string {
  const kind = documentKindLabel(item);
  if (item.type === "file") {
    const size = formatFileSize(item.size);
    return size ? `${kind} · ${size}` : kind;
  }
  return kind;
}

function IconShare() {
  return (
    <svg viewBox="0 0 24 24" className="w-3.5 h-3.5" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path
        d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function IconMove() {
  return (
    <svg viewBox="0 0 24 24" className="w-3.5 h-3.5" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M5 9l-3 3 3 3M19 15l3-3-3-3M2 12h20" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function IconTrash() {
  return (
    <svg viewBox="0 0 24 24" className="w-3.5 h-3.5" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function IconLeave() {
  return (
    <svg viewBox="0 0 24 24" className="w-3.5 h-3.5" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4M16 17l5-5-5-5M21 12H9" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function IconPeople() {
  return (
    <svg viewBox="0 0 24 24" className="w-3.5 h-3.5" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path
        d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2M9 11a4 4 0 100-8 4 4 0 000 8zM23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

async function collectDroppedFiles(dataTransfer: DataTransfer): Promise<DropFile[]> {
  const items = dataTransfer.items;
  if (!items || items.length === 0) {
    return Array.from(dataTransfer.files).map((file) => ({ file, relPath: file.name }));
  }

  const out: DropFile[] = [];

  const readAllEntries = (reader: FileSystemDirectoryReader): Promise<FileSystemEntry[]> =>
    new Promise((resolve, reject) => {
      const acc: FileSystemEntry[] = [];
      const readBatch = () => {
        reader.readEntries((batch) => {
          if (!batch.length) resolve(acc);
          else {
            acc.push(...batch);
            readBatch();
          }
        }, reject);
      };
      readBatch();
    });

  const walkEntry = async (entry: FileSystemEntry, basePath: string): Promise<void> => {
    if (entry.isFile) {
      const file = await new Promise<File>((resolve, reject) => {
        (entry as FileSystemFileEntry).file(resolve, reject);
      });
      const relPath = basePath ? `${basePath}/${file.name}` : file.name;
      out.push({ file, relPath });
      return;
    }
    if (entry.isDirectory) {
      const dirPath = basePath ? `${basePath}/${entry.name}` : entry.name;
      const reader = (entry as FileSystemDirectoryEntry).createReader();
      const entries = await readAllEntries(reader);
      for (const child of entries) {
        await walkEntry(child, dirPath);
      }
    }
  };

  const tasks: Promise<void>[] = [];
  for (let i = 0; i < items.length; i++) {
    const entry = items[i].webkitGetAsEntry?.();
    if (entry) {
      tasks.push(walkEntry(entry, ""));
    } else if (items[i].kind === "file") {
      const file = items[i].getAsFile();
      if (file) out.push({ file, relPath: file.name });
    }
  }
  await Promise.all(tasks);

  if (out.length === 0) {
    return Array.from(dataTransfer.files).map((file) => ({ file, relPath: file.name }));
  }
  return out;
}

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

  const handleDrop = async (e: React.DragEvent) => {
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
      (scope === "shared" && isShareOwner && shareId));
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
    <main className="flex flex-col gap-5 p-4 sm:p-6 w-full mx-auto max-w-[1200px] sm:pt-[6vh]">
      <input
        ref={fileInputRef}
        type="file"
        multiple
        className="hidden"
        onChange={async (e) => {
          const list = Array.from(e.target.files ?? []).map((file) => ({ file, relPath: file.name }));
          e.target.value = "";
          await uploadFiles(list);
        }}
      />
      <section className="bg-white border border-gray-200 shadow-sm p-5 rounded-2xl flex flex-col lg:flex-row gap-4 lg:items-center lg:justify-between">
        <div data-tour="documents-intro">
          <h1 className="text-xl font-bold text-gray-900">Cloud personnel</h1>
          <p className="text-sm text-gray-500 mt-1">
            Cloud personnel et dossiers partagés avec le personnel Clerk.
          </p>
          {quota && (
            <div data-tour="documents-quota" className="mt-3">
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
        <div data-tour="documents-upload" className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => setShowNewFolder(true)}
            disabled={isSharePicker || isInIncomingSharedFolder}
            className="px-4 py-2 rounded-xl border border-gray-300 text-sm font-semibold hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed"
          >
            + Dossier
          </button>
          <button
            type="button"
            onClick={() => fileInputRef.current?.click()}
            disabled={isSharePicker || isInIncomingSharedFolder || uploading}
            className="px-4 py-2 rounded-xl border border-gray-300 text-sm font-semibold hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed"
          >
            + Ajouter un fichier
          </button>
          <button
            type="button"
            onClick={() => setShowNewShare(true)}
            className="px-4 py-2 rounded-xl border border-blue-200 text-blue-700 text-sm font-semibold hover:bg-blue-50"
          >
            + Dossier partagé
          </button>
          {activeShare && shareId && (
            <button
              type="button"
              onClick={() => openShareAccess(activeShare)}
              className="px-4 py-2 rounded-xl border border-indigo-200 text-indigo-700 text-sm font-semibold hover:bg-indigo-50"
            >
              Voir qui a accès
            </button>
          )}
          {activeShare?.isOwner && shareId && (
            <button
              type="button"
              onClick={() => {
                setManageMembers(activeShare.memberIds);
                setShowShareManage(true);
              }}
              className="px-4 py-2 rounded-xl bg-slate-800 text-white text-sm font-semibold hover:bg-slate-700"
            >
              Modifier les accès
            </button>
          )}
          {canDeleteCurrentFolder && (
            <button
              type="button"
              onClick={() => {
                setDeleteFolderConfirm("");
                setShowDeleteFolder(true);
              }}
              className="px-4 py-2 rounded-xl border border-red-200 text-red-700 text-sm font-semibold hover:bg-red-50"
            >
              {deleteFolderLabel}
            </button>
          )}
          {scope === "shared" && shareId && !isShareOwner && (
            <button
              type="button"
              onClick={() => setShowLeaveShare(true)}
              className="px-4 py-2 rounded-xl border border-amber-200 text-amber-800 text-sm font-semibold hover:bg-amber-50"
            >
              Quitter le dossier
            </button>
          )}
        </div>
      </section>

      <div className="flex flex-col md:flex-row gap-4 md:items-stretch">
        <aside data-tour="documents-scope" className="md:w-56 shrink-0 bg-white border border-gray-200 rounded-2xl p-3 flex flex-col max-h-[min(720px,calc(100vh-10rem))] md:min-h-[480px]">
          <button
            type="button"
            onClick={goToPersonalRoot}
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
            onClick={openIncomingSharedFiles}
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
                    openShare(share);
                  }}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    setSidebarShareMenu({ share, x: e.clientX, y: e.clientY });
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

        <section className="flex-1 min-w-0 flex flex-col min-h-[480px] max-h-[min(720px,calc(100vh-10rem))]">
          <div data-tour="documents-breadcrumb" className="flex items-center gap-1 text-sm text-gray-500 mb-3 flex-wrap">
            <button type="button" onClick={() => navigateToSegment(-1)} className="hover:text-blue-600 font-medium">
              {rootLabel}
            </button>
            {pathSegments.map((seg, i) => (
              <span key={`${seg}-${i}`} className="flex items-center gap-1">
                <span>/</span>
                <button type="button" onClick={() => navigateToSegment(i)} className="hover:text-blue-600 font-medium">
                  {seg}
                </button>
              </span>
            ))}
          </div>

          {error && (
            <div className="mb-3 rounded-xl border border-red-200 bg-red-50 px-4 py-2 text-sm text-red-700">
              {error}
            </div>
          )}

          {isSharePicker ? (
            <div className="rounded-3xl border border-dashed border-gray-200 bg-gray-50/80 p-10 text-center text-gray-500">
              <p className="text-4xl mb-3">👥</p>
              <p className="font-medium">Sélectionnez un dossier partagé dans la barre latérale.</p>
            </div>
          ) : (
            <div
              data-tour="documents-dropzone"
              className={[
                "relative flex flex-col flex-1 min-h-0 rounded-3xl border-2 border-dashed transition-colors",
                dragActive ? "border-blue-500 bg-blue-50/50" : "border-gray-200 bg-gray-50/40",
                uploading ? "opacity-70 pointer-events-none" : "",
              ].join(" ")}
              onDragEnter={(e) => {
                e.preventDefault();
                e.stopPropagation();
                if (!uploading) setDragActive(true);
              }}
              onDragOver={(e) => {
                e.preventDefault();
                e.stopPropagation();
                if (!uploading) setDragActive(true);
              }}
              onDragLeave={(e) => {
                e.preventDefault();
                e.stopPropagation();
                if (e.currentTarget === e.target) setDragActive(false);
              }}
              onDrop={handleDrop}
            >
              {loading && (
                <div className="absolute top-4 right-4">
                  <div className="h-5 w-5 border-2 border-blue-600 border-t-transparent animate-spin rounded-full" />
                </div>
              )}

              {uploading && (
                <div className="absolute inset-0 flex items-center justify-center bg-white/60 rounded-3xl z-10">
                  <p className="text-sm font-semibold text-blue-700">Envoi en cours…</p>
                </div>
              )}

              {dragActive && !uploading && (
                <div className="absolute inset-0 flex items-center justify-center pointer-events-none z-10">
                  <p className="text-blue-700 font-bold text-lg bg-white/90 px-6 py-3 rounded-2xl shadow">
                    Déposez vos fichiers ou dossiers ici
                  </p>
                </div>
              )}

              <div className="flex-1 min-h-0 overflow-y-auto p-4">
              {items.length > 0 ? (
                <div
                  data-tour="documents-grid"
                  className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 gap-2 sm:gap-3 items-start"
                  onClick={(e) => e.stopPropagation()}
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
                </div>
              ) : (
                !loading && (
                  <div className="flex flex-col items-center justify-center min-h-[240px] h-full text-gray-400 pointer-events-none">
                    <span className="text-4xl mb-3">📂</span>
                    <p className="font-medium text-gray-600 text-sm">Ce dossier est vide</p>
                    <p className="text-xs mt-2 italic text-center px-4">
                      Glissez-déposez des fichiers ou dossiers ici
                    </p>
                  </div>
                )
              )}
              </div>
            </div>
          )}
        </section>
      </div>

      {showDeleteFolder && (
        <Modal title={deleteFolderLabel} onClose={() => setShowDeleteFolder(false)}>
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
        </Modal>
      )}

      {showLeaveShare && activeShare && (
        <Modal title="Quitter le dossier partagé" onClose={() => setShowLeaveShare(false)}>
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
        </Modal>
      )}

      {shareFileItem && (
        <Modal
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
        </Modal>
      )}

      {showNewFolder && (
        <Modal title="Nouveau dossier" onClose={() => setShowNewFolder(false)}>
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
        </Modal>
      )}

      {showNewShare && (
        <Modal title="Nouveau dossier partagé" onClose={() => setShowNewShare(false)} wide>
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
        </Modal>
      )}

      {moveItem && (
        <Modal title={`Déplacer « ${moveItem.name}${moveItem.ext ? `.${moveItem.ext}` : ""} »`} onClose={() => setMoveItem(null)} wide>
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
        </Modal>
      )}

      {showShareManage && activeShare && (
        <Modal title={`Modifier les accès — ${activeShare.name}`} onClose={() => setShowShareManage(false)} wide>
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
        </Modal>
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
      <ReplayModuleTourButton moduleId="documents" />
    </main>
  );
}

function DocumentContextMenu({
  x,
  y,
  onClose,
  onOpen,
  onShare,
  onMove,
  onDelete,
  onLeave,
  onShowAccess,
}: {
  x: number;
  y: number;
  onClose: () => void;
  onOpen: () => void;
  onShare?: () => void;
  onMove?: () => void;
  onDelete?: () => void;
  onLeave?: () => void;
  onShowAccess?: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    // Mac Ctrl+clic : un `click` suit juste après et atterrirait sur « Ouvrir ».
    let done = false;
    const arm = () => {
      if (done) return;
      done = true;
      setArmed(true);
    };
    const swallowGhostClick = (e: MouseEvent) => {
      if (done) return;
      e.preventDefault();
      e.stopPropagation();
      arm();
    };
    window.addEventListener("click", swallowGhostClick, true);
    // Clic droit Windows / trackpad : souvent pas de click fantôme
    const t = window.setTimeout(arm, 50);
    return () => {
      window.removeEventListener("click", swallowGhostClick, true);
      window.clearTimeout(t);
    };
  }, []);

  useEffect(() => {
    if (!armed) return;
    const onPointer = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onPointer);
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", onClose, true);
    return () => {
      window.removeEventListener("mousedown", onPointer);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", onClose, true);
    };
  }, [armed, onClose]);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pad = 8;
    let left = x;
    let top = y;
    if (left + rect.width > window.innerWidth - pad) left = window.innerWidth - rect.width - pad;
    if (top + rect.height > window.innerHeight - pad) top = window.innerHeight - rect.height - pad;
    el.style.left = `${Math.max(pad, left)}px`;
    el.style.top = `${Math.max(pad, top)}px`;
  }, [x, y]);

  const run = (fn: () => void) => {
    onClose();
    fn();
  };

  const itemClass =
    "flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-slate-700 hover:bg-slate-50 transition-colors";

  return createPortal(
    <div
      ref={ref}
      className="fixed z-[200] min-w-[11.5rem] overflow-hidden rounded-xl border border-slate-200 bg-white py-1 shadow-2xl shadow-slate-900/15"
      style={{ left: x, top: y, pointerEvents: armed ? "auto" : "none" }}
      role="menu"
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {/* Zone morte : le click fantôme Mac (Ctrl+clic) atterrit ici, pas sur « Ouvrir ». */}
      <div className="h-1.5 w-full" aria-hidden />
      <button type="button" className={itemClass} role="menuitem" onClick={() => run(onOpen)}>
        <span className="text-base leading-none" aria-hidden>↗</span>
        Ouvrir
      </button>
      {onShowAccess ? (
        <button
          type="button"
          className={`${itemClass} text-indigo-700`}
          role="menuitem"
          onClick={() => run(onShowAccess)}
        >
          <IconPeople />
          Voir qui a accès
        </button>
      ) : null}
      {onShare ? (
        <button type="button" className={`${itemClass} text-indigo-700`} role="menuitem" onClick={() => run(onShare)}>
          <IconShare />
          Partager
        </button>
      ) : null}
      {onMove ? (
        <button type="button" className={`${itemClass} text-blue-700`} role="menuitem" onClick={() => run(onMove)}>
          <IconMove />
          Déplacer
        </button>
      ) : null}
      {onDelete ? (
        <button type="button" className={`${itemClass} text-red-600`} role="menuitem" onClick={() => run(onDelete)}>
          <IconTrash />
          Supprimer
        </button>
      ) : null}
      {onLeave ? (
        <button type="button" className={`${itemClass} text-amber-700`} role="menuitem" onClick={() => run(onLeave)}>
          <IconLeave />
          Retirer du partage
        </button>
      ) : null}
    </div>,
    document.body,
  );
}

function ShareSidebarContextMenu({
  x,
  y,
  share,
  onClose,
  onOpen,
  onShowAccess,
  onManageAccess,
}: {
  x: number;
  y: number;
  share: ShareInfo;
  onClose: () => void;
  onOpen: () => void;
  onShowAccess: () => void;
  onManageAccess?: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    let done = false;
    const arm = () => {
      if (done) return;
      done = true;
      setArmed(true);
    };
    const swallowGhostClick = (e: MouseEvent) => {
      if (done) return;
      e.preventDefault();
      e.stopPropagation();
      arm();
    };
    window.addEventListener("click", swallowGhostClick, true);
    const t = window.setTimeout(arm, 50);
    return () => {
      window.removeEventListener("click", swallowGhostClick, true);
      window.clearTimeout(t);
    };
  }, []);

  useEffect(() => {
    if (!armed) return;
    const onPointer = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onPointer);
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", onClose, true);
    return () => {
      window.removeEventListener("mousedown", onPointer);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", onClose, true);
    };
  }, [armed, onClose]);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pad = 8;
    let left = x;
    let top = y;
    if (left + rect.width > window.innerWidth - pad) left = window.innerWidth - rect.width - pad;
    if (top + rect.height > window.innerHeight - pad) top = window.innerHeight - rect.height - pad;
    el.style.left = `${Math.max(pad, left)}px`;
    el.style.top = `${Math.max(pad, top)}px`;
  }, [x, y]);

  const run = (fn: () => void) => {
    onClose();
    fn();
  };

  const itemClass =
    "flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-slate-700 hover:bg-slate-50 transition-colors";

  return createPortal(
    <div
      ref={ref}
      className="fixed z-[200] min-w-[12rem] overflow-hidden rounded-xl border border-slate-200 bg-white py-1 shadow-2xl shadow-slate-900/15"
      style={{ left: x, top: y, pointerEvents: armed ? "auto" : "none" }}
      role="menu"
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      <div className="h-1.5 w-full" aria-hidden />
      <p className="px-3 py-1.5 text-[10px] font-bold uppercase tracking-wide text-slate-400 truncate">
        {share.name}
      </p>
      <button type="button" className={itemClass} role="menuitem" onClick={() => run(onOpen)}>
        <span className="text-base leading-none" aria-hidden>↗</span>
        Ouvrir
      </button>
      <button
        type="button"
        className={`${itemClass} text-indigo-700`}
        role="menuitem"
        onClick={() => run(onShowAccess)}
      >
        <IconPeople />
        Voir qui a accès
      </button>
      {onManageAccess ? (
        <button
          type="button"
          className={`${itemClass} text-slate-800`}
          role="menuitem"
          onClick={() => run(onManageAccess)}
        >
          <IconShare />
          Modifier les accès
        </button>
      ) : null}
    </div>,
    document.body,
  );
}

function AccessModal({
  title,
  people,
  onClose,
}: {
  title: string;
  people: AccessPerson[];
  onClose: () => void;
}) {
  return (
    <Modal title={`Qui a accès — ${title}`} onClose={onClose} wide>
      <p className="text-sm text-gray-500 mb-3">
        Personnel Clerk autorisé à consulter ce partage.
      </p>
      <ul className="divide-y divide-gray-100 border border-gray-200 rounded-xl overflow-hidden max-h-72 overflow-y-auto">
        {people.map((person) => (
          <li key={person.userId} className="px-4 py-3 bg-white">
            <p className="text-sm font-semibold text-gray-900">
              {person.isOwner ? "👑 " : ""}
              {person.name}
              {person.isYou ? <span className="text-gray-500 font-normal"> (vous)</span> : null}
              {person.isOwner ? (
                <span className="ml-2 text-[10px] font-bold uppercase tracking-wide text-amber-700 bg-amber-50 px-1.5 py-0.5 rounded">
                  Propriétaire
                </span>
              ) : null}
            </p>
            <p className="text-xs text-gray-500 mt-0.5">{person.detail}</p>
          </li>
        ))}
      </ul>
      <div className="flex justify-end mt-4">
        <button
          type="button"
          onClick={onClose}
          className="px-4 py-2 text-sm rounded-xl bg-slate-800 text-white font-semibold"
        >
          Fermer
        </button>
      </div>
    </Modal>
  );
}

function DocumentItemCard({
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

function MoveDestBreadcrumb({
  rootLabel,
  path,
  onNavigate,
}: {
  rootLabel: string;
  path: string;
  onNavigate: (path: string) => void;
}) {
  const segments = path.replace(/\/$/, "").split("/").filter(Boolean);
  return (
    <div className="flex items-center gap-1 text-sm text-gray-500 mb-3 flex-wrap">
      <button type="button" onClick={() => onNavigate("")} className="hover:text-blue-600 font-medium">
        {rootLabel}
      </button>
      {segments.map((seg, i) => (
        <span key={`${seg}-${i}`} className="flex items-center gap-1">
          <span>/</span>
          <button
            type="button"
            onClick={() => onNavigate(`${segments.slice(0, i + 1).join("/")}/`)}
            className="hover:text-blue-600 font-medium"
          >
            {seg}
          </button>
        </span>
      ))}
    </div>
  );
}

function Modal({
  title,
  children,
  onClose,
  wide,
}: {
  title: string;
  children: React.ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-sm" onClick={onClose}>
      <div
        className={`bg-white rounded-2xl shadow-xl p-5 w-full ${wide ? "max-w-lg" : "max-w-md"}`}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="text-lg font-bold text-gray-900 mb-4">{title}</h2>
        {children}
      </div>
    </div>
  );
}

function PeerPicker({
  peers,
  search,
  onSearch,
  selected,
  onToggle,
}: {
  peers: Peer[];
  search: string;
  onSearch: (v: string) => void;
  selected: string[];
  onToggle: (id: string) => void;
}) {
  return (
    <div>
      <input
        value={search}
        onChange={(e) => onSearch(e.target.value)}
        placeholder="Rechercher une personne…"
        className="w-full border border-gray-300 rounded-xl px-3 py-2 text-sm mb-2"
      />
      <div className="max-h-52 overflow-y-auto border border-gray-200 rounded-xl divide-y">
        {peers.length === 0 ? (
          <p className="p-3 text-sm text-gray-400 italic">Aucune personne trouvée.</p>
        ) : (
          peers.map((p) => (
            <label key={p.clerkUserId} className="flex items-center gap-3 px-3 py-2 hover:bg-gray-50 cursor-pointer text-sm">
              <input
                type="checkbox"
                checked={selected.includes(p.clerkUserId)}
                onChange={() => onToggle(p.clerkUserId)}
              />
              <span className="flex-1 min-w-0">
                <span className="font-medium text-gray-800 block truncate">{peerFullName(p)}</span>
                <span className="text-xs text-gray-500 truncate block">{peerRoleLabels(p)}</span>
              </span>
            </label>
          ))
        )}
      </div>
      <p className="text-xs text-gray-400 mt-2">{selected.length} personne(s) sélectionnée(s)</p>
    </div>
  );
}
