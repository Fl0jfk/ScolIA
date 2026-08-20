import "server-only";

import { getJson, putJson, putObject, getObjectBytes } from "@/app/lib/s3-storage";
import { isInscriptionLevelId } from "@/app/lib/document-templates/inscription-levels";
import type {
  InscriptionLevelId,
  InscriptionTenantSettings,
} from "@/app/lib/document-templates/types";

const SETTINGS_KEY = "documents/inscription/settings.json";

export function defaultInscriptionTenantSettings(): InscriptionTenantSettings {
  return {
    establishmentName: "",
    accentColor: "#0f172a",
    overrides: {},
  };
}

export function inscriptionOverrideKey(levelId: InscriptionLevelId): string {
  return `documents/inscription/overrides/${levelId}.pdf`;
}

export async function loadInscriptionTenantSettings(): Promise<InscriptionTenantSettings> {
  const hit = await getJson<InscriptionTenantSettings>(SETTINGS_KEY);
  const base = defaultInscriptionTenantSettings();
  if (!hit?.data || typeof hit.data !== "object") return base;
  const d = hit.data;
  const overrides: InscriptionTenantSettings["overrides"] = {};
  if (d.overrides && typeof d.overrides === "object") {
    for (const [k, v] of Object.entries(d.overrides)) {
      if (isInscriptionLevelId(k) && typeof v === "string" && v.trim()) {
        overrides[k] = v.trim();
      }
    }
  }
  const color = String(d.accentColor || base.accentColor).trim();
  return {
    establishmentName: String(d.establishmentName || "").slice(0, 120),
    accentColor: /^#[0-9a-fA-F]{3,8}$/.test(color) ? color : base.accentColor,
    overrides,
    updatedAt: typeof d.updatedAt === "string" ? d.updatedAt : undefined,
  };
}

export async function saveInscriptionTenantSettings(
  partial: Partial<InscriptionTenantSettings>,
): Promise<InscriptionTenantSettings> {
  const current = await loadInscriptionTenantSettings();
  const next: InscriptionTenantSettings = {
    establishmentName:
      partial.establishmentName !== undefined
        ? String(partial.establishmentName).slice(0, 120)
        : current.establishmentName,
    accentColor:
      partial.accentColor && /^#[0-9a-fA-F]{3,8}$/.test(partial.accentColor.trim())
        ? partial.accentColor.trim()
        : current.accentColor,
    overrides:
      partial.overrides !== undefined
        ? (() => {
            const overrides: InscriptionTenantSettings["overrides"] = {};
            for (const [k, v] of Object.entries(partial.overrides)) {
              if (isInscriptionLevelId(k) && typeof v === "string" && v.trim()) {
                overrides[k] = v.trim();
              }
            }
            return overrides;
          })()
        : current.overrides,
    updatedAt: new Date().toISOString(),
  };
  await putJson(SETTINGS_KEY, next);
  return next;
}

export async function saveInscriptionLevelOverride(
  levelId: InscriptionLevelId,
  bytes: Buffer,
): Promise<{ key: string }> {
  const key = inscriptionOverrideKey(levelId);
  await putObject(key, bytes, "application/pdf");
  const settings = await loadInscriptionTenantSettings();
  await saveInscriptionTenantSettings({
    overrides: { ...settings.overrides, [levelId]: key },
  });
  return { key };
}

export async function clearInscriptionLevelOverride(
  levelId: InscriptionLevelId,
): Promise<InscriptionTenantSettings> {
  const settings = await loadInscriptionTenantSettings();
  const overrides = { ...settings.overrides };
  delete overrides[levelId];
  return saveInscriptionTenantSettings({ overrides });
}

export async function loadInscriptionOverrideBytes(
  levelId: InscriptionLevelId,
): Promise<Buffer | null> {
  const settings = await loadInscriptionTenantSettings();
  const key = settings.overrides[levelId];
  if (!key) return null;
  return getObjectBytes(key);
}
