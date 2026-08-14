import "server-only";

import { ensureFolderPath } from "@/app/lib/graph-onedrive-folders";
import { GRAPH_API_BASE, graphDriveRootItemUrl } from "@/app/lib/graph-onedrive-path";

function parseGraphRetryAfterMs(res: Response, attempt: number): number {
  const raw = res.headers.get("Retry-After");
  if (raw) {
    const sec = Number(raw);
    if (!Number.isNaN(sec) && sec > 0) return sec * 1000;
  }
  return Math.min(60_000, 4000 * 2 ** attempt);
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function graphConflict(detail: string, status: number): boolean {
  return (
    status === 412 ||
    status === 409 ||
    detail.includes("resourceModified") ||
    detail.includes("nameAlreadyExists") ||
    detail.includes("eTag")
  );
}

/** Vérifie si un fichier existe déjà à ce chemin OneDrive. */
async function oneDriveItemExists(
  accessToken: string,
  itemPath: string,
): Promise<boolean> {
  const res = await fetch(graphDriveRootItemUrl(itemPath), {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  return res.ok;
}

async function uploadBytesToOneDrive(
  accessToken: string,
  itemPath: string,
  bytes: Uint8Array | Buffer,
  contentType = "application/pdf",
): Promise<{ ok: true } | { ok: false; status: number; detail: string }> {
  const body = bytes instanceof Buffer ? new Uint8Array(bytes) : bytes;

  for (let attempt = 0; attempt < 5; attempt++) {
    const replace = attempt === 0;
    const url = replace
      ? graphDriveRootItemUrl(itemPath, "/content?@microsoft.graph.conflictBehavior=replace")
      : graphDriveRootItemUrl(itemPath, "/content");

    const res = await fetch(url, {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": contentType,
      },
      body: body as BodyInit,
    });

    if (res.status === 429) {
      await sleep(parseGraphRetryAfterMs(res, attempt));
      continue;
    }

    if (!res.ok) {
      const detail = (await res.text()).slice(0, 500);
      if (graphConflict(detail, res.status) && attempt < 4) {
        await deleteOneDrivePath(accessToken, itemPath);
        await sleep(400 + attempt * 300);
        continue;
      }
      return { ok: false, status: res.status, detail };
    }
    return { ok: true };
  }
  return { ok: false, status: 429, detail: "Limite OneDrive atteinte" };
}

export async function deleteOneDrivePath(accessToken: string, itemPath: string) {
  try {
    const res = await fetch(graphDriveRootItemUrl(itemPath), {
      method: "DELETE",
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    if (!res.ok && res.status !== 404) {
      console.warn("[ocr-graph] delete", itemPath, res.status);
    }
  } catch (e) {
    console.warn("[ocr-graph] delete", itemPath, e);
  }
}

export async function moveOneDriveFile(
  accessToken: string,
  sourcePath: string,
  targetFolderPath: string,
  newFileName?: string,
): Promise<{ ok: true; finalFileName: string } | { ok: false; status: number; detail: string }> {
  const sourceRes = await fetch(graphDriveRootItemUrl(sourcePath), {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!sourceRes.ok) {
    return { ok: false, status: sourceRes.status, detail: await sourceRes.text() };
  }
  const sourceItem = await sourceRes.json();
  const sourceItemId = sourceItem.id as string | undefined;
  const driveId = sourceItem.parentReference?.driveId as string | undefined;
  if (!sourceItemId || !driveId) {
    return { ok: false, status: 500, detail: "ID source ou drive manquant" };
  }

  await ensureFolderPath(accessToken, targetFolderPath);

  const targetFolderRes = await fetch(graphDriveRootItemUrl(targetFolderPath), {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!targetFolderRes.ok) {
    return { ok: false, status: targetFolderRes.status, detail: await targetFolderRes.text() };
  }
  const targetFolder = await targetFolderRes.json();
  const targetFolderId = targetFolder.id as string | undefined;
  if (!targetFolderId) {
    return { ok: false, status: 500, detail: "ID dossier cible manquant" };
  }

  const finalFileName =
    newFileName && newFileName.trim().length > 0 ? newFileName.trim() : (sourceItem.name as string);

  const childrenRes = await fetch(`${GRAPH_API_BASE}/drives/${driveId}/items/${targetFolderId}/children`, {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let children: any[] = [];
  if (childrenRes.ok) {
    const childrenJson = await childrenRes.json();
    children = childrenJson.value || [];
  }

  const dotIndex = finalFileName.lastIndexOf(".");
  const base = dotIndex > 0 ? finalFileName.slice(0, dotIndex) : finalFileName;
  const ext = dotIndex > 0 ? finalFileName.slice(dotIndex) : "";
  let safeName = finalFileName;
  let suffix = 2;
  while (children.some((c) => c.name === safeName)) {
    safeName = `${base} (${suffix})${ext}`;
    suffix++;
  }

  for (let attempt = 0; attempt < 6; attempt++) {
    const moveRes = await fetch(`${GRAPH_API_BASE}/drives/${driveId}/items/${sourceItemId}`, {
      method: "PATCH",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        parentReference: { id: targetFolderId },
        name: safeName,
      }),
    });
    if (moveRes.status === 429) {
      await sleep(parseGraphRetryAfterMs(moveRes, attempt));
      continue;
    }
    if (!moveRes.ok) {
      const detail = await moveRes.text();
      if (graphConflict(detail, moveRes.status) && attempt < 5) {
        suffix++;
        safeName = `${base} (${suffix})${ext}`;
        await sleep(300 + attempt * 200);
        continue;
      }
      return { ok: false, status: moveRes.status, detail };
    }
    return { ok: true, finalFileName: safeName };
  }
  return { ok: false, status: 429, detail: "Limite OneDrive lors du déplacement" };
}

/** Dépose un PDF dans un dossier élève avec suffixe (2), (3)… si le nom existe déjà. */
export async function uploadBytesToOneDriveUnique(
  accessToken: string,
  folderPath: string,
  baseFileName: string,
  bytes: Uint8Array | Buffer,
): Promise<{ ok: true; path: string; fileName: string } | { ok: false; status: number; detail: string }> {
  const folder = folderPath.replace(/\/+$/, "");
  const raw = baseFileName.trim() || "Document.pdf";
  const dot = raw.lastIndexOf(".");
  const base = dot > 0 ? raw.slice(0, dot) : raw;
  const ext = dot > 0 ? raw.slice(dot) : ".pdf";

  for (let n = 0; n < 8; n++) {
    const fileName = n === 0 ? `${base}${ext}` : `${base} (${n + 1})${ext}`;
    const itemPath = `${folder}/${fileName}`;
    const res = await uploadBytesToOneDrive(accessToken, itemPath, bytes);
    if (res.ok) return { ok: true, path: itemPath, fileName };
    if (!graphConflict(res.detail, res.status)) {
      return res;
    }
  }
  return { ok: false, status: 409, detail: "Impossible de déposer le fichier (conflits de nom)" };
}
