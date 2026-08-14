import { INTRANET_ROLE_OPTIONS } from "@/app/lib/intranet-roles";

export type DocumentScope = "personal" | "shared";

export type DocumentItem = {
  type: "folder" | "file";
  name: string;
  relPath: string;
  ext?: string;
  size?: number;
  sharedBy?: string;
  isVirtual?: boolean;
};

export const INCOMING_SHARED_FILES_FOLDER = "Fichiers partagés";
export const FILE_SHARE_REL_PREFIX = "__fileshare__/";

export type ShareInfo = {
  id: string;
  name: string;
  ownerId: string;
  memberIds: string[];
  isOwner: boolean;
};

export type Peer = {
  clerkUserId: string;
  firstName?: string;
  lastName?: string;
  email: string;
  roles: string[];
};

export type FileShareMetaBrief = {
  id: string;
  ownerId: string;
  memberIds: string[];
  sourceRelPath: string;
  fileName: string;
  ext?: string;
};

export type AccessPerson = {
  userId: string;
  name: string;
  detail: string;
  isOwner: boolean;
  isYou: boolean;
};

export type QuotaInfo = {
  used: number;
  quota: number;
  usedLabel: string;
  quotaLabel: string;
  percent: number;
};

export type DropFile = { file: File; relPath: string };

/** Mac Ctrl+clic = menu contextuel : ne pas traiter comme un clic d'ouverture. */
export function isMacContextClick(e: { ctrlKey: boolean; button: number; metaKey?: boolean }) {
  return e.ctrlKey || e.button === 2;
}

export function isVirtualFileSharePath(relPath: string) {
  return relPath.startsWith(FILE_SHARE_REL_PREFIX);
}

export function fileShareIdFromPath(relPath: string) {
  return relPath.slice(FILE_SHARE_REL_PREFIX.length).replace(/\/$/, "");
}

export function peerFullName(p: Peer): string {
  const name = `${p.firstName ?? ""} ${p.lastName ?? ""}`.trim();
  return name || "Nom non renseigné";
}

export function peerRoleLabels(p: Peer): string {
  if (!p.roles.length) return "Aucun rôle";
  return p.roles
    .map((slug) => INTRANET_ROLE_OPTIONS.find((o) => o.slug === slug)?.label ?? slug)
    .join(", ");
}

export function normalizeDocPath(path: string): string {
  return path.replace(/^\/+/, "").replace(/\/+/g, "/");
}

export function buildAccessPeople(
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

export function resolveItemAccess(
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

export function formatFileSize(bytes?: number): string | null {
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

export function documentDisplayName(item: DocumentItem): string {
  if (item.type === "file" && item.ext) return `${item.name}.${item.ext}`;
  return item.name;
}

export function documentKindLabel(item: DocumentItem): string {
  if (item.name === INCOMING_SHARED_FILES_FOLDER) return "Dossier";
  if (item.type === "folder") return item.isVirtual ? "Dossier partagé" : "Dossier";
  return item.isVirtual ? "Fichier partagé" : "Fichier";
}

export function documentAccent(item: DocumentItem, folderVariant?: "shared-incoming" | "default"): string {
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

export function documentMetaLine(item: DocumentItem): string {
  const kind = documentKindLabel(item);
  if (item.type === "file") {
    const size = formatFileSize(item.size);
    return size ? `${kind} · ${size}` : kind;
  }
  return kind;
}

export async function collectDroppedFiles(dataTransfer: DataTransfer): Promise<DropFile[]> {
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
