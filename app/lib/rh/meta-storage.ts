import "server-only";

import { graphDriveRootItemUrl } from "@/app/lib/graph-onedrive-path";
import {
  getRhDriveAccessToken,
  getRhDrivePublicConfig,
} from "@/app/lib/rh/graph-rh-drive";
import {
  emptyRhPersonnelIndex,
  normalizeMetaRhDocument,
  normalizeRhPersonnelIndex,
  type MetaRhDocument,
  type RhPersonnelIndex,
} from "@/app/lib/rh/types";
import { rhIndexPath, rhMetaPath } from "@/app/lib/rh/paths";

async function graphGetJson<T>(
  accessToken: string,
  pathInDrive: string,
): Promise<{ ok: true; data: T } | { ok: false; status: number; error: string }> {
  const url = graphDriveRootItemUrl(pathInDrive, "/content");
  const res = await fetch(url, {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (res.status === 404) {
    return { ok: false, status: 404, error: "not_found" };
  }
  if (!res.ok) {
    return { ok: false, status: res.status, error: (await res.text()).slice(0, 200) };
  }
  try {
    const data = (await res.json()) as T;
    return { ok: true, data };
  } catch {
    return { ok: false, status: 500, error: "json_parse_error" };
  }
}

async function graphPutJson(
  accessToken: string,
  pathInDrive: string,
  body: unknown,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const url = graphDriveRootItemUrl(pathInDrive, "/content");
  const res = await fetch(url, {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json; charset=utf-8",
    },
    body: JSON.stringify(body, null, 2),
  });
  if (!res.ok) {
    return { ok: false, error: (await res.text()).slice(0, 240) };
  }
  return { ok: true };
}

export async function readRhPersonnelIndex(): Promise<
  { ok: true; index: RhPersonnelIndex; basePath: string } | { ok: false; error: string; code: string }
> {
  const token = await getRhDriveAccessToken();
  if ("error" in token) return { ok: false, error: token.error, code: token.code };

  const path = rhIndexPath(token.basePath);
  const hit = await graphGetJson<unknown>(token.accessToken, path);
  if (!hit.ok) {
    if (hit.status === 404) {
      return {
        ok: true,
        index: emptyRhPersonnelIndex(token.basePath),
        basePath: token.basePath,
      };
    }
    return { ok: false, error: hit.error, code: "RH_INDEX_READ_ERROR" };
  }
  return {
    ok: true,
    index: normalizeRhPersonnelIndex(hit.data, token.basePath),
    basePath: token.basePath,
  };
}

export async function writeRhPersonnelIndex(
  index: RhPersonnelIndex,
): Promise<{ ok: true } | { ok: false; error: string; code: string }> {
  const token = await getRhDriveAccessToken();
  if ("error" in token) return { ok: false, error: token.error, code: token.code };

  const next: RhPersonnelIndex = {
    ...index,
    version: 1,
    basePath: token.basePath,
    updatedAt: new Date().toISOString(),
  };
  const put = await graphPutJson(token.accessToken, rhIndexPath(token.basePath), next);
  if (!put.ok) return { ok: false, error: put.error, code: "RH_INDEX_WRITE_ERROR" };
  return { ok: true };
}

export async function readMetaRhByFolderName(
  folderName: string,
): Promise<
  { ok: true; meta: MetaRhDocument } | { ok: false; error: string; code: string; status?: number }
> {
  const token = await getRhDriveAccessToken();
  if ("error" in token) return { ok: false, error: token.error, code: token.code };

  const path = rhMetaPath(folderName, token.basePath);
  const hit = await graphGetJson<unknown>(token.accessToken, path);
  if (!hit.ok) {
    return {
      ok: false,
      error: hit.status === 404 ? "meta-rh.json introuvable" : hit.error,
      code: hit.status === 404 ? "RH_META_NOT_FOUND" : "RH_META_READ_ERROR",
      status: hit.status,
    };
  }
  return { ok: true, meta: normalizeMetaRhDocument(hit.data) };
}

export async function writeMetaRh(
  folderName: string,
  meta: MetaRhDocument,
): Promise<{ ok: true; meta: MetaRhDocument } | { ok: false; error: string; code: string }> {
  const token = await getRhDriveAccessToken();
  if ("error" in token) return { ok: false, error: token.error, code: token.code };

  const next: MetaRhDocument = {
    ...normalizeMetaRhDocument(meta),
    updatedAt: new Date().toISOString(),
  };
  const put = await graphPutJson(token.accessToken, rhMetaPath(folderName, token.basePath), next);
  if (!put.ok) return { ok: false, error: put.error, code: "RH_META_WRITE_ERROR" };
  return { ok: true, meta: next };
}

export async function ensureRhDriveLinked(): Promise<
  { ok: true; basePath: string } | { ok: false; error: string; code: string }
> {
  const cfg = await getRhDrivePublicConfig();
  if (!cfg.linked) {
    return {
      ok: false,
      error: "OneDrive RH non connecté.",
      code: "RH_DRIVE_NOT_LINKED",
    };
  }
  const token = await getRhDriveAccessToken();
  if ("error" in token) return { ok: false, error: token.error, code: token.code };
  return { ok: true, basePath: token.basePath };
}
