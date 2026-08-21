import { GRAPH_API_BASE, graphDriveRootItemUrl } from "@/app/lib/graph-onedrive-path";

async function graphGetItemByPath(accessToken: string, itemPath: string) {
  const res = await fetch(graphDriveRootItemUrl(itemPath), {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!res.ok) return null;
  return res.json();
}

/** Liste les noms des sous-dossiers directs d'un chemin (une page, jusqu'à 500). */
export async function listChildFolderNames(accessToken: string, folderPath: string): Promise<Set<string>> {
  const names = new Set<string>();
  const url = graphDriveRootItemUrl(folderPath, "/children?$select=name,folder&$top=500");
  const res = await fetch(url, { headers: { Authorization: `Bearer ${accessToken}` } });
  if (!res.ok) return names;
  const data = await res.json();
  for (const item of data.value ?? []) {
    if (item.folder && item.name) names.add(String(item.name));
  }
  return names;
}

export type OneDriveFileChild = {
  id: string;
  name: string;
  eTag?: string;
  size?: number;
  lastModifiedDateTime?: string;
};

/** Fichiers (pas dossiers) à la racine d'un chemin OneDrive. */
export async function listChildFiles(
  accessToken: string,
  folderPath: string,
): Promise<OneDriveFileChild[]> {
  const out: OneDriveFileChild[] = [];
  let url: string | undefined = graphDriveRootItemUrl(
    folderPath,
    "/children?$select=id,name,file,eTag,size,lastModifiedDateTime&$top=200",
  );
  while (url) {
    const res = await fetch(url, { headers: { Authorization: `Bearer ${accessToken}` } });
    if (!res.ok) break;
    const data = (await res.json()) as {
      value?: Array<Record<string, unknown>>;
      "@odata.nextLink"?: string;
    };
    for (const item of data.value ?? []) {
      if (item.file && item.name) {
        out.push({
          id: String(item.id ?? ""),
          name: String(item.name),
          eTag: item.eTag ? String(item.eTag) : undefined,
          size: typeof item.size === "number" ? item.size : undefined,
          lastModifiedDateTime: item.lastModifiedDateTime
            ? String(item.lastModifiedDateTime)
            : undefined,
        });
      }
    }
    url = data["@odata.nextLink"];
  }
  return out;
}

export async function downloadOneDriveFileBytes(
  accessToken: string,
  filePath: string,
): Promise<Uint8Array | null> {
  const res = await fetch(graphDriveRootItemUrl(filePath, "/content"), {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!res.ok) return null;
  const buf = new Uint8Array(await res.arrayBuffer());
  return buf;
}

/** Crée un dossier enfant sous parentPath s'il n'existe pas. */
export async function ensureChildFolder(
  accessToken: string,
  parentPath: string,
  folderName: string,
): Promise<{ created: boolean; existed: boolean; path: string }> {
  const safeName = folderName.replace(/[\\/:*?"<>|]+/g, "_").trim();
  const fullPath = parentPath ? `${parentPath.replace(/\/+$/, "")}/${safeName}` : safeName;
  const existing = await graphGetItemByPath(accessToken, fullPath);
  if (existing?.id) return { created: false, existed: true, path: fullPath };

  const parent = parentPath.replace(/\/+$/, "") || "";
  const createUrl = parent
    ? graphDriveRootItemUrl(parent, "/children")
    : `${GRAPH_API_BASE}/me/drive/root/children`;

  const res = await fetch(createUrl, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ name: safeName, folder: {}, "@microsoft.graph.conflictBehavior": "fail" }),
  });
  if (!res.ok) {
    const err = await res.text();
    throw new Error(`Création dossier « ${safeName} » : ${err}`);
  }
  return { created: true, existed: false, path: fullPath };
}

/** Crée toute la chaîne de dossiers (ex. Dossier élèves / Lycée). */
export async function ensureFolderPath(accessToken: string, fullPath: string): Promise<void> {
  const parts = fullPath.split("/").filter(Boolean);
  let acc = "";
  for (const part of parts) {
    await ensureChildFolder(accessToken, acc, part);
    acc = acc ? `${acc}/${part}` : part;
  }
}

/** Évite les collisions de noms dans un dossier OneDrive. */
async function resolveUniqueFileName(
  accessToken: string,
  folderPath: string,
  fileName: string,
): Promise<string> {
  const children = await listChildFolderNames(accessToken, folderPath);
  if (!children.has(fileName)) return fileName;
  const dot = fileName.lastIndexOf(".");
  const base = dot > 0 ? fileName.slice(0, dot) : fileName;
  const ext = dot > 0 ? fileName.slice(dot) : "";
  let suffix = 2;
  let candidate = `${base} (${suffix})${ext}`;
  while (children.has(candidate)) {
    suffix += 1;
    candidate = `${base} (${suffix})${ext}`;
  }
  return candidate;
}

/** Upload binaire dans un dossier OneDrive (crée le dossier si besoin). */
export async function uploadFileToOneDriveFolder(
  accessToken: string,
  folderPath: string,
  fileName: string,
  bytes: Uint8Array,
  contentType = "application/pdf",
): Promise<{ folderPath: string; fileName: string; fullPath: string }> {
  await ensureFolderPath(accessToken, folderPath);
  const safeName = await resolveUniqueFileName(accessToken, folderPath, fileName);
  const fullPath = `${folderPath.replace(/\/+$/, "")}/${safeName}`;
  const res = await fetch(graphDriveRootItemUrl(fullPath, "/content"), {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": contentType,
    },
    body: bytes as BodyInit,
  });
  if (!res.ok) {
    const err = await res.text();
    throw new Error(`Upload OneDrive échoué : ${err}`);
  }
  return { folderPath, fileName: safeName, fullPath };
}
