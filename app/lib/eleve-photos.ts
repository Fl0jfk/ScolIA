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

const PHOTO_INDEX_KEY = "eleves/photo-index.json";

export type ElevePhotoIndex = Record<string, string>;

export async function loadElevePhotoIndex(): Promise<ElevePhotoIndex> {
  const hit = await getJson<ElevePhotoIndex>(PHOTO_INDEX_KEY);
  if (!hit?.data || typeof hit.data !== "object") return {};
  return hit.data;
}

function lookupS3Key(
  index: ElevePhotoIndex,
  person: { nom: string; prenom: string; ine?: string | null; photoKey?: string | null },
): string | null {
  if (person.photoKey) return person.photoKey;
  const ine = person.ine?.trim().toUpperCase();
  if (ine && index[`ine:${ine}`]) return index[`ine:${ine}`]!;
  const nameKey = `name:${identityKey(person.nom, person.prenom)}`;
  return index[nameKey] ?? null;
}

export async function getElevePhotoUrl(eleve: EleveConfig): Promise<string | null> {
  const index = await loadElevePhotoIndex();
  const key = lookupS3Key(index, eleve);
  if (!key) return null;
  try {
    return await getSignedReadUrl(key, 60 * 60);
  } catch {
    return null;
  }
}

/** URLs signées pour l’appel internat (id élève → URL). */
export async function resolvePhotoUrlsForInternatStudents(
  students: InternatStudent[],
): Promise<Record<string, string>> {
  const index = await loadElevePhotoIndex();
  const out: Record<string, string> = {};
  const active = students.filter((s) => s.actif);

  await Promise.all(
    active.map(async (s) => {
      const key = lookupS3Key(index, {
        nom: s.eleveRef.nom,
        prenom: s.eleveRef.prenom,
        ine: s.eleveRef.ine,
      });
      if (!key) return;
      try {
        const url = await getSignedReadUrl(key, 60 * 60);
        if (url) out[s.id] = url;
      } catch {
        /* ignore missing object */
      }
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
      try {
        const url = await getSignedReadUrl(key, 60 * 60);
        if (url) out[e.id] = url;
      } catch {
        /* ignore */
      }
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

/** Enregistre des photos + index ; photoKey persisté sur le référentiel élèves (table eleve). */
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
  if (fromRegistry && updated > 0) {
    try {
      await saveElevesRegistry(nextEleves);
    } catch (e) {
      console.warn("[eleve-photos] saveElevesRegistry (photoKey best-effort)", e);
    }
  }

  return { matched, unmatched, updated };
}
