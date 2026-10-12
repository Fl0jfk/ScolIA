import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  elevePhotoLookupKey,
  identityKey,
  matchEleveForPhoto,
  parsePhotoFilename,
  photoRelativePathForEleve,
} from "@/app/lib/eleve-photos-match";
import { loadElevesRegistry, saveElevesRegistry } from "@/app/lib/eleves-registry";
import { getInternatStudents } from "@/app/lib/internat-storage";
import type { InternatStudent } from "@/app/lib/internat-types";
import { getJson, getSignedReadUrl, putJson, putObject } from "@/app/lib/s3-storage";

export {
  elevePhotoLookupKey,
  identityKey,
  matchEleveForPhoto,
  parsePhotoFilename,
  photoRelativePathForEleve,
} from "@/app/lib/eleve-photos-match";

export function elevePhotoProxyPath(eleveId: string): string {
  return `/api/eleves/${encodeURIComponent(eleveId)}/photo`;
}

export function resolveElevePhotoS3Key(
  index: ElevePhotoIndex,
  person: { nom: string; prenom: string; ine?: string | null; photoKey?: string | null },
): string | null {
  return lookupS3Key(index, person);
}

const PHOTO_INDEX_KEY = "eleves/photo-index.json";
const PHOTO_INDEX_CACHE_MS = 45_000;
const SIGNED_URL_CACHE_MS = 50 * 60 * 1000; // signatures ~1h — on garde 50 min

export type ElevePhotoIndex = Record<string, string>;

let photoIndexCacheByTenant = new Map<string, { at: number; data: ElevePhotoIndex }>();
const signedUrlCache = new Map<string, { url: string; exp: number }>();

async function photoIndexCacheKey(): Promise<string> {
  const { resolveCacheTenantSlug } = await import("@/app/lib/cache-tenant-key");
  return resolveCacheTenantSlug();
}

export async function loadElevePhotoIndex(): Promise<ElevePhotoIndex> {
  const key = await photoIndexCacheKey();
  const hitMem = photoIndexCacheByTenant.get(key);
  if (hitMem && Date.now() - hitMem.at < PHOTO_INDEX_CACHE_MS) {
    return hitMem.data;
  }
  try {
    const hit = await getJson<ElevePhotoIndex>(PHOTO_INDEX_KEY);
    const data =
      hit?.data && typeof hit.data === "object" ? hit.data : ({} as ElevePhotoIndex);
    photoIndexCacheByTenant.set(key, { at: Date.now(), data });
    return data;
  } catch (e) {
    console.warn("[eleve-photos] loadElevePhotoIndex", e);
    return {};
  }
}

export function invalidateElevePhotoIndexCache(): void {
  photoIndexCacheByTenant.clear();
}

export async function saveElevePhotoIndex(index: ElevePhotoIndex): Promise<void> {
  await putJson(PHOTO_INDEX_KEY, index);
  invalidateElevePhotoIndexCache();
}

async function signedUrlCached(key: string, expiresIn = 60 * 60): Promise<string | null> {
  const now = Date.now();
  const hit = signedUrlCache.get(key);
  if (hit && hit.exp > now) return hit.url;
  try {
    const url = await getSignedReadUrl(key, expiresIn);
    if (url) {
      signedUrlCache.set(key, { url, exp: now + SIGNED_URL_CACHE_MS });
    }
    return url;
  } catch {
    return null;
  }
}

/** Clés d’index photo pour une identité (INE + nom/prénom + ordre inversé). */
export function photoIndexKeysForPerson(person: {
  nom: string;
  prenom: string;
  ine?: string | null;
}): string[] {
  const keys: string[] = [];
  const ine = person.ine?.trim().toUpperCase();
  if (ine) keys.push(`ine:${ine}`);
  const forward = identityKey(person.nom, person.prenom);
  const reverse = identityKey(person.prenom, person.nom);
  if (forward) keys.push(`name:${forward}`);
  if (reverse && reverse !== forward) keys.push(`name:${reverse}`);
  return keys;
}

function lookupS3Key(
  index: ElevePhotoIndex,
  person: { nom: string; prenom: string; ine?: string | null; photoKey?: string | null },
): string | null {
  if (person.photoKey) return person.photoKey;
  for (const key of photoIndexKeysForPerson(person)) {
    const hit = index[key];
    if (hit) return hit;
  }
  return null;
}

export async function getElevePhotoUrl(eleve: EleveConfig): Promise<string | null> {
  const index = await loadElevePhotoIndex();
  const key = lookupS3Key(index, eleve);
  if (!key) return null;
  return signedUrlCached(key, 60 * 60);
}

/** Map studentId → clé S3 photo (index eleves/photo-index.json). */
export async function photoS3KeysForInternatStudents(
  students: InternatStudent[],
  index?: ElevePhotoIndex,
): Promise<Map<string, string>> {
  const idx = index ?? (await loadElevePhotoIndex());
  const out = new Map<string, string>();
  for (const s of students) {
    const key = lookupS3Key(idx, {
      nom: s.eleveRef.nom,
      prenom: s.eleveRef.prenom,
      ine: s.eleveRef.ine,
    });
    if (key) out.set(s.id, key);
  }
  return out;
}

/**
 * Après fusion de doublons internat : réécrit l’index photo pour que toutes les
 * identités (INE / noms des fiches absorbées + fiche conservée) pointent vers
 * la même photo S3 — sinon la photo « disparaît » avec l’id/nom abandonné.
 */
export async function aliasPhotoIndexAfterInternatMerge(opts: {
  mergeTraces: Array<{ keeperId: string; absorbedIds: string[]; members: InternatStudent[] }>;
  photoS3KeyByStudentId: ReadonlyMap<string, string>;
}): Promise<{ aliased: number }> {
  if (!opts.mergeTraces.length) return { aliased: 0 };
  const index = await loadElevePhotoIndex();
  let aliased = 0;
  let dirty = false;

  for (const trace of opts.mergeTraces) {
    let s3Key =
      opts.photoS3KeyByStudentId.get(trace.keeperId) ||
      trace.absorbedIds.map((id) => opts.photoS3KeyByStudentId.get(id)).find(Boolean) ||
      null;
    if (!s3Key) {
      for (const m of trace.members) {
        s3Key = lookupS3Key(index, {
          nom: m.eleveRef.nom,
          prenom: m.eleveRef.prenom,
          ine: m.eleveRef.ine,
        });
        if (s3Key) break;
      }
    }
    if (!s3Key) continue;

    for (const m of trace.members) {
      for (const key of photoIndexKeysForPerson({
        nom: m.eleveRef.nom,
        prenom: m.eleveRef.prenom,
        ine: m.eleveRef.ine,
      })) {
        if (index[key] !== s3Key) {
          index[key] = s3Key;
          aliased += 1;
          dirty = true;
        }
      }
    }
  }

  if (dirty) await saveElevePhotoIndex(index);
  return { aliased };
}

/** URLs signées pour l’appel / fiches internat (id élève → URL). */
export async function resolvePhotoUrlsForInternatStudents(
  students: InternatStudent[],
): Promise<Record<string, string>> {
  const index = await loadElevePhotoIndex();
  const out: Record<string, string> = {};

  await Promise.all(
    students.map(async (s) => {
      const key = lookupS3Key(index, {
        nom: s.eleveRef.nom,
        prenom: s.eleveRef.prenom,
        ine: s.eleveRef.ine,
      });
      if (!key) return;
      const url = await signedUrlCached(key, 60 * 60);
      if (url) out[s.id] = url;
    }),
  );

  return out;
}

/** URLs signées pour un lot d'élèves (id → URL) — appels de classe VS. */
export async function resolvePhotoUrlsForEleves(
  eleves: Array<{
    id: string;
    nom: string;
    prenom: string;
    ine?: string | null;
    photoKey?: string | null;
  }>,
): Promise<Record<string, string>> {
  const index = await loadElevePhotoIndex();
  const out: Record<string, string> = {};

  await Promise.all(
    eleves.map(async (e) => {
      const key = lookupS3Key(index, e);
      if (!key) return;
      const url = await signedUrlCached(key, 60 * 60);
      if (url) out[e.id] = url;
    }),
  );

  return out;
}

async function elevesForPhotoMatch(): Promise<{ eleves: EleveConfig[]; fromRegistry: boolean }> {
  const registry = await loadElevesRegistry();
  if (registry.length) return { eleves: registry, fromRegistry: true };

  const students = await getInternatStudents();
  const eleves: EleveConfig[] = students
    .filter((s) => s.actif)
    .map((s) => ({
      ine: s.eleveRef.ine || "",
      nom: s.eleveRef.nom,
      prenom: s.eleveRef.prenom,
      folderName: s.eleveRef.folderName,
    }));
  return { eleves, fromRegistry: false };
}

export type PhotoBulkResult = {
  matched: number;
  unmatched: string[];
  updated: number;
};

/**
 * Enregistre des photos + index ; photoKey persisté sur le référentiel élèves (table eleve).
 * Écrase la photo S3 existante pour le même élève (même clé).
 * Préférer le flux batch (`eleve-photos-batch`) pour les gros volumes.
 */
export async function applyElevePhotosBulk(
  files: { filename: string; bytes: Uint8Array; contentType: string }[],
): Promise<PhotoBulkResult> {
  const { eleves, fromRegistry } = await elevesForPhotoMatch();
  if (!eleves.length) {
    throw new Error(
      "Aucun élève pour associer les photos — importez d’abord le référentiel élèves (Paramètres → Liste des élèves).",
    );
  }

  const index = await loadElevePhotoIndex();
  const unmatched: string[] = [];
  let matched = 0;
  let updated = 0;
  const nextEleves = [...eleves];

  for (const file of files) {
    const parsed = parsePhotoFilename(file.filename);
    if (!parsed) {
      unmatched.push(file.filename);
      continue;
    }
    const eleve = matchEleveForPhoto(nextEleves, parsed.nom, parsed.prenom);
    if (!eleve) {
      unmatched.push(file.filename);
      continue;
    }
    matched += 1;
    const relative = photoRelativePathForEleve(eleve);
    const ct = file.contentType.startsWith("image/") ? file.contentType : "image/jpeg";
    const s3Key = await putObject(relative, file.bytes, ct);
    const lookup = elevePhotoLookupKey(eleve);
    index[lookup] = s3Key;
    index[`name:${identityKey(eleve.nom, eleve.prenom)}`] = s3Key;

    const idx = nextEleves.findIndex(
      (e) =>
        (eleve.ine && e.ine && e.ine === eleve.ine) ||
        identityKey(e.nom, e.prenom) === identityKey(eleve.nom, eleve.prenom),
    );
    if (idx >= 0) {
      nextEleves[idx] = { ...nextEleves[idx]!, photoKey: s3Key };
      updated += 1;
    }
  }

  await putJson(PHOTO_INDEX_KEY, index);
  invalidateElevePhotoIndexCache();
  if (fromRegistry && updated > 0) {
    try {
      await saveElevesRegistry(nextEleves);
    } catch (e) {
      console.warn("[eleve-photos] saveElevesRegistry (photoKey best-effort)", e);
    }
  }

  return { matched, unmatched, updated };
}
