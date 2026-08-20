import "server-only";

import { getJson, putJson, putObject, getObjectBytes } from "@/app/lib/s3-storage";
import { isInscriptionLevelId } from "@/app/lib/document-templates/inscription-levels";
import {
  defaultSixiemeCodeConfig,
  normalizeSixiemeCodeConfig,
} from "@/app/lib/document-templates/inscription-sixieme-config";
import type {
  InscriptionLevelCodeConfig,
  InscriptionLevelId,
  InscriptionPdfFontId,
  InscriptionTenantSettings,
} from "@/app/lib/document-templates/types";

const SETTINGS_KEY = "documents/inscription/settings.json";

function normalizePdfFont(raw: unknown): InscriptionPdfFontId {
  const v = String(raw || "")
    .trim()
    .toLowerCase();
  if (v === "helvetica" || v === "courier" || v === "times") return v;
  return "times";
}

export function defaultInscriptionTenantSettings(): InscriptionTenantSettings {
  return {
    establishmentName: "",
    accentColor: "#1E4A32",
    pdfFont: "times",
    overrides: {},
    levelConfigs: {
      sixieme: defaultSixiemeCodeConfig(),
    },
  };
}

export function inscriptionOverrideKey(levelId: InscriptionLevelId): string {
  return `documents/inscription/overrides/${levelId}.pdf`;
}

function parseLevelConfigs(
  raw: unknown,
): InscriptionTenantSettings["levelConfigs"] {
  if (!raw || typeof raw !== "object") {
    return { sixieme: defaultSixiemeCodeConfig() };
  }
  const o = raw as Record<string, unknown>;
  const out: NonNullable<InscriptionTenantSettings["levelConfigs"]> = {};
  if (o.sixieme) out.sixieme = normalizeSixiemeCodeConfig(o.sixieme);
  else out.sixieme = defaultSixiemeCodeConfig();
  return out;
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
    pdfFont: normalizePdfFont(d.pdfFont),
    overrides,
    levelConfigs: parseLevelConfigs(d.levelConfigs),
    updatedAt: typeof d.updatedAt === "string" ? d.updatedAt : undefined,
  };
}

export async function saveInscriptionTenantSettings(
  partial: Partial<InscriptionTenantSettings>,
): Promise<InscriptionTenantSettings> {
  const current = await loadInscriptionTenantSettings();
  const nextLevelConfigs = { ...(current.levelConfigs || {}) };
  if (partial.levelConfigs) {
    if (partial.levelConfigs.sixieme) {
      nextLevelConfigs.sixieme = normalizeSixiemeCodeConfig(partial.levelConfigs.sixieme);
    }
  }

  const next: InscriptionTenantSettings = {
    establishmentName:
      partial.establishmentName !== undefined
        ? String(partial.establishmentName).slice(0, 120)
        : current.establishmentName,
    accentColor:
      partial.accentColor && /^#[0-9a-fA-F]{3,8}$/.test(partial.accentColor.trim())
        ? partial.accentColor.trim()
        : current.accentColor,
    pdfFont:
      partial.pdfFont !== undefined ? normalizePdfFont(partial.pdfFont) : current.pdfFont || "times",
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
    levelConfigs: nextLevelConfigs,
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

export function getSixiemeConfigFromSettings(
  settings: InscriptionTenantSettings,
): InscriptionLevelCodeConfig {
  return normalizeSixiemeCodeConfig(settings.levelConfigs?.sixieme);
}
