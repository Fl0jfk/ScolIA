import "server-only";

import { getPersonnelIndex } from "@/app/lib/personnel-storage";
import { buildPersonnelFolderName, type RhPersonnelIndexEntry } from "@/app/lib/rh/types";
import { readRhPersonnelIndex } from "@/app/lib/rh/meta-storage";

function splitDisplayName(displayName: string): { nom: string; prenom: string } {
  const parts = displayName.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return { nom: displayName.trim(), prenom: "" };
  if (parts.length === 1) return { nom: parts[0], prenom: "" };
  const last = parts[parts.length - 1];
  const firstIsAllCaps = parts[0] === parts[0].toUpperCase() && /[A-ZÀ-Ÿ]/.test(parts[0]);
  if (firstIsAllCaps) {
    return { nom: parts[0], prenom: parts.slice(1).join(" ") };
  }
  return { nom: last, prenom: parts.slice(0, -1).join(" ") };
}

function fromS3Entry(entry: {
  id: string;
  displayName: string;
  email: string;
  clerkUserId?: string | null;
  active: boolean;
  category: string;
}): RhPersonnelIndexEntry {
  const { nom, prenom } = splitDisplayName(entry.displayName);
  return {
    id: entry.id,
    folderName: buildPersonnelFolderName(nom, prenom) || entry.displayName,
    displayName: entry.displayName,
    email: entry.email,
    clerkUserId: entry.clerkUserId,
    category: (entry.category as RhPersonnelIndexEntry["category"]) || "administratif",
    active: entry.active !== false,
    accountStatus: "active",
  };
}

/** Index personnel pour l’OCR : OneDrive RH si lié, sinon registre S3 OGEC. */
export async function loadPersonnelEntriesForOcr(): Promise<RhPersonnelIndexEntry[]> {
  try {
    const rh = await readRhPersonnelIndex();
    if (rh.ok && rh.index.entries.length > 0) {
      return rh.index.entries.filter((e) => e.active !== false);
    }
  } catch {
    /* rhDrive absent ou illisible — repli S3 */
  }
  try {
    const s3 = await getPersonnelIndex();
    return s3.filter((e) => e.active !== false).map(fromS3Entry);
  } catch {
    return [];
  }
}
