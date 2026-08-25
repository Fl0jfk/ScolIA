import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  parseRentreeLinks,
  parseRentreePages,
  syncRentreePages,
} from "@/app/lib/rentree-pages";
import { RENTREE_LINKS } from "@/app/lib/rentree-defaults";
import {
  parseFournituresProfiles,
  parseFournituresToolConfig,
} from "@/app/lib/fournitures-config";
import {
  defaultToolboxConfig,
  type ToolboxConfig,
  type ToolboxToolId,
} from "@/app/lib/toolbox-types";

const TOOLBOX_KEY = "settings/modules/toolbox.json";

function numRecord(v: unknown): Record<string, number> {
  if (!v || typeof v !== "object") return {};
  const out: Record<string, number> = {};
  for (const [k, val] of Object.entries(v as Record<string, unknown>)) {
    const n = Number(val);
    if (Number.isFinite(n)) out[k] = n;
  }
  return out;
}

function numArr(v: unknown, len = 5): number[] {
  if (!Array.isArray(v)) return [];
  return v.map((x) => Number(x)).filter((n) => Number.isFinite(n)).slice(0, len);
}

function parseSlots(raw: unknown) {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((item, i) => {
      const o = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
      const id = String(o.id || `slot-${i + 1}`).trim();
      const label = String(o.label || `Créneau ${i + 1}`).trim();
      const startAt = String(o.startAt || "").trim();
      const endAt = String(o.endAt || "").trim();
      if (!startAt) return null;
      const maxPlaces = Number(o.maxPlaces);
      return {
        id,
        label,
        startAt,
        endAt: endAt || startAt,
        maxPlaces: Number.isFinite(maxPlaces) && maxPlaces > 0 ? maxPlaces : undefined,
      };
    })
    .filter((x): x is NonNullable<typeof x> => Boolean(x));
}

export function parseToolboxConfig(raw: unknown): ToolboxConfig {
  const defaults = defaultToolboxConfig();
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const tools = o.tools && typeof o.tools === "object" ? (o.tools as Record<string, unknown>) : {};

  const qr = tools.qrcreator && typeof tools.qrcreator === "object" ? (tools.qrcreator as Record<string, unknown>) : {};
  const santa =
    tools["secret-santa"] && typeof tools["secret-santa"] === "object"
      ? (tools["secret-santa"] as Record<string, unknown>)
      : {};
  const rentree =
    tools.rentree && typeof tools.rentree === "object" ? (tools.rentree as Record<string, unknown>) : {};
  const tarifs =
    tools["simulateur-tarifs"] && typeof tools["simulateur-tarifs"] === "object"
      ? (tools["simulateur-tarifs"] as Record<string, unknown>)
      : {};
  const fournitures =
    tools["simulateur-fournitures"] && typeof tools["simulateur-fournitures"] === "object"
      ? (tools["simulateur-fournitures"] as Record<string, unknown>)
      : {};
  const po =
    tools["portes-ouvertes"] && typeof tools["portes-ouvertes"] === "object"
      ? (tools["portes-ouvertes"] as Record<string, unknown>)
      : {};
  const repartition =
    tools["repartition-classes"] && typeof tools["repartition-classes"] === "object"
      ? (tools["repartition-classes"] as Record<string, unknown>)
      : {};
  const covoiturage =
    tools.covoiturage && typeof tools.covoiturage === "object"
      ? (tools.covoiturage as Record<string, unknown>)
      : {};

  const ens = tarifs.enseignement && typeof tarifs.enseignement === "object" ? tarifs.enseignement : {};
  const demi = tarifs.demiPension && typeof tarifs.demiPension === "object" ? tarifs.demiPension : {};

  return {
    tools: {
      qrcreator: { enabled: qr.enabled !== false },
      "secret-santa": {
        enabled: santa.enabled === true,
        title: String(santa.title || defaults.tools["secret-santa"].title).trim(),
        budgetHint: String(santa.budgetHint || defaults.tools["secret-santa"].budgetHint).trim(),
        participantNames: Array.isArray(santa.participantNames)
          ? santa.participantNames.map((n) => String(n).trim()).filter(Boolean)
          : [],
      },
      rentree: {
        enabled: rentree.enabled === true,
        title: String(rentree.title || defaults.tools.rentree.title).trim(),
        schoolYear: String(rentree.schoolYear || defaults.tools.rentree.schoolYear).trim(),
        showSimulateurTarifs: rentree.showSimulateurTarifs !== false,
        showSimulateurFournitures: rentree.showSimulateurFournitures !== false,
        pages: parseRentreePages(rentree.pages),
        links: parseRentreeLinks(rentree.links),
      },
      "simulateur-tarifs": {
        enabled: tarifs.enabled === true,
        schoolYear: String(tarifs.schoolYear || defaults.tools["simulateur-tarifs"].schoolYear).trim(),
        enseignement: {
          maternelle: numArr((ens as Record<string, unknown>).maternelle).length
            ? numArr((ens as Record<string, unknown>).maternelle)
            : defaults.tools["simulateur-tarifs"].enseignement.maternelle,
          elementaire: numArr((ens as Record<string, unknown>).elementaire).length
            ? numArr((ens as Record<string, unknown>).elementaire)
            : defaults.tools["simulateur-tarifs"].enseignement.elementaire,
          college: numArr((ens as Record<string, unknown>).college).length
            ? numArr((ens as Record<string, unknown>).college)
            : defaults.tools["simulateur-tarifs"].enseignement.college,
          lycee: numArr((ens as Record<string, unknown>).lycee).length
            ? numArr((ens as Record<string, unknown>).lycee)
            : defaults.tools["simulateur-tarifs"].enseignement.lycee,
        },
        demiPension: {
          maternelle: Object.keys(numRecord((demi as Record<string, unknown>).maternelle)).length
            ? numRecord((demi as Record<string, unknown>).maternelle)
            : defaults.tools["simulateur-tarifs"].demiPension.maternelle,
          elementaire: Object.keys(numRecord((demi as Record<string, unknown>).elementaire)).length
            ? numRecord((demi as Record<string, unknown>).elementaire)
            : defaults.tools["simulateur-tarifs"].demiPension.elementaire,
          college: Object.keys(numRecord((demi as Record<string, unknown>).college)).length
            ? numRecord((demi as Record<string, unknown>).college)
            : defaults.tools["simulateur-tarifs"].demiPension.college,
          lycee: Object.keys(numRecord((demi as Record<string, unknown>).lycee)).length
            ? numRecord((demi as Record<string, unknown>).lycee)
            : defaults.tools["simulateur-tarifs"].demiPension.lycee,
        },
        pensionAnnuel: Number.isFinite(Number(tarifs.pensionAnnuel))
          ? Number(tarifs.pensionAnnuel)
          : defaults.tools["simulateur-tarifs"].pensionAnnuel,
        garderie: Object.keys(numRecord(tarifs.garderie)).length
          ? numRecord(tarifs.garderie)
          : defaults.tools["simulateur-tarifs"].garderie,
      },
      "simulateur-fournitures": parseFournituresToolConfig(fournitures, defaults.tools["simulateur-fournitures"]),
      "portes-ouvertes": {
        enabled: po.enabled === true,
        title: String(po.title || defaults.tools["portes-ouvertes"].title).trim(),
        intro: String(po.intro || defaults.tools["portes-ouvertes"].intro).trim(),
        address: String(po.address || "").trim(),
        mapsUrl: String(po.mapsUrl || "").trim() || undefined,
        notifyEmail: String(po.notifyEmail || "").trim() || undefined,
        slots: parseSlots(po.slots),
        consentLabel: String(po.consentLabel || defaults.tools["portes-ouvertes"].consentLabel).trim(),
      },
      "repartition-classes": {
        enabled: repartition.enabled !== false,
        label: String(repartition.label || defaults.tools["repartition-classes"].label).trim(),
      },
      covoiturage: {
        enabled: covoiturage.enabled === true,
        label: String(covoiturage.label || defaults.tools.covoiturage.label).trim(),
      },
    },
  };
}

export async function getToolboxConfig(): Promise<ToolboxConfig> {
  const raw = await getJson<unknown>(TOOLBOX_KEY);
  if (!raw?.data) return defaultToolboxConfig();
  return parseToolboxConfig(raw.data);
}

/** Configuration boîte à outils avec pages rentrée alignées sur les établissements actifs. */
export async function getToolboxConfigResolved(): Promise<ToolboxConfig> {
  const [config, app] = await Promise.all([getToolboxConfig(), loadAppConfig()]);
  return resolveToolboxRentreePages(config, app.establishments);
}

export function resolveToolboxRentreePages(
  config: ToolboxConfig,
  establishments: import("@/app/lib/app-config-schemas").Establishment[],
): ToolboxConfig {
  const rentree = config.tools.rentree;
  const legacy = rentree.links?.length ? rentree.links : RENTREE_LINKS;
  const pages = syncRentreePages(establishments, rentree.pages, legacy);
  return {
    ...config,
    tools: {
      ...config.tools,
      rentree: {
        ...rentree,
        pages,
      },
    },
  };
}

export async function saveToolboxConfig(config: ToolboxConfig): Promise<void> {
  const app = await loadAppConfig();
  const resolved = resolveToolboxRentreePages(config, app.establishments);
  const toSave = {
    ...resolved,
    tools: {
      ...resolved.tools,
      rentree: {
        ...resolved.tools.rentree,
        links: undefined,
      },
    },
  };
  await putJson(TOOLBOX_KEY, toSave);
}

export function toolboxEnabledTools(config: ToolboxConfig): ToolboxToolId[] {
  const ids: ToolboxToolId[] = [];
  if (config.tools.qrcreator.enabled) ids.push("qrcreator");
  // événements → /etablissement/evenements ; tarifs → Communication ; fournitures → Rentrée
  if (config.tools["repartition-classes"].enabled) ids.push("repartition-classes");
  if (config.tools.covoiturage.enabled) ids.push("covoiturage");
  return ids;
}

/** Covoiturage activé dans la boîte à outils (désactivé par défaut). */
export async function isCovoiturageToolEnabled(): Promise<boolean> {
  const config = await getToolboxConfig();
  return config.tools.covoiturage.enabled === true;
}
