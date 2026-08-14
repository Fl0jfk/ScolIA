import { getJson, putJson } from "@/app/lib/s3-storage";
import { ESTABLISHMENT_KIND_PRESETS } from "@/app/lib/establishment-visual";
import {
  ORGANIGRAM_ACCOUNTING,
  ORGANIGRAM_ADMIN,
  ORGANIGRAM_DIRECTORS,
  ORGANIGRAM_HEALTH,
  ORGANIGRAM_MAINTENANCE,
  ORGANIGRAM_OGEC,
  ORGANIGRAM_PASTORAL,
  ORGANIGRAM_POLES,
  ORGANIGRAM_RECEPTION,
  ORGANIGRAM_TUTELLE,
  type OrganigramPerson,
} from "@/app/lib/organigramme";

const ORGANIGRAMME_CONFIG_KEY = "settings/organigramme.json";

export type OrganigramSectionId =
  | "direction"
  | "admin"
  | "accounting"
  | "poles"
  | "reception"
  | "health"
  | "maintenance"
  | "pastoral"
  | "ogec"
  | "tutelle";

export type OrganigramSlot = {
  id: string;
  sectionId: OrganigramSectionId;
  poleId?: string;
  blockId?: string;
  order: number;
  email?: string;
  personnelId?: string;
  establishmentId?: string;
  firstName?: string;
  lastName?: string;
  role?: string;
  missions?: string[];
  photoUrl?: string | null;
  hidden?: boolean;
};

export type OrganigramSectionMeta = {
  id: OrganigramSectionId;
  title: string;
  description?: string;
  visible?: boolean;
};

export type OrganigramPoleMeta = {
  id: string;
  label: string;
  blocks: Array<{ id: string; title: string; description?: string }>;
};

export type OrganigramConfig = {
  version: 1;
  sections: OrganigramSectionMeta[];
  poles: OrganigramPoleMeta[];
  slots: OrganigramSlot[];
};

function personToSlot(
  person: OrganigramPerson,
  sectionId: OrganigramSectionId,
  order: number,
  extra?: Partial<OrganigramSlot>,
): OrganigramSlot {
  return {
    id: person.id,
    sectionId,
    order,
    email: person.email,
    firstName: person.firstName,
    lastName: person.lastName,
    role: person.role,
    missions: [...(person.missions || [])],
    photoUrl: person.photoUrl ?? "",
    ...extra,
  };
}

/** Seed = organigramme hardcodé actuel (migration one-shot). */
function seedOrganigramConfig(): OrganigramConfig {
  const sections: OrganigramSectionMeta[] = [
    {
      id: "direction",
      title: "Direction du groupe scolaire",
      description:
        "Les trois directions de l'école, du collège et du lycée pilotent chacune leur cycle et coordonnent les projets communs.",
      visible: true,
    },
    {
      id: "admin",
      title: ORGANIGRAM_ADMIN.title,
      description: ORGANIGRAM_ADMIN.description,
      visible: true,
    },
    {
      id: "accounting",
      title: ORGANIGRAM_ACCOUNTING.title,
      description: ORGANIGRAM_ACCOUNTING.description,
      visible: true,
    },
    {
      id: "poles",
      title: "Pôles éducatifs & vie scolaire",
      description:
        "Équipes par cycle — CPE et accompagnement, distincts du seul pôle administratif.",
      visible: true,
    },
    {
      id: "reception",
      title: ORGANIGRAM_RECEPTION.title,
      description: ORGANIGRAM_RECEPTION.description,
      visible: true,
    },
    {
      id: "health",
      title: ORGANIGRAM_HEALTH.title,
      description: ORGANIGRAM_HEALTH.description,
      visible: true,
    },
    {
      id: "maintenance",
      title: ORGANIGRAM_MAINTENANCE.title,
      description: ORGANIGRAM_MAINTENANCE.description,
      visible: true,
    },
    {
      id: "pastoral",
      title: ORGANIGRAM_PASTORAL.title,
      description: ORGANIGRAM_PASTORAL.description,
      visible: true,
    },
    {
      id: "ogec",
      title: ORGANIGRAM_OGEC.title,
      description: ORGANIGRAM_OGEC.description,
      visible: true,
    },
    {
      id: "tutelle",
      title: ORGANIGRAM_TUTELLE.title,
      description: ORGANIGRAM_TUTELLE.description,
      visible: true,
    },
  ];

  const poles: OrganigramPoleMeta[] = ORGANIGRAM_POLES.map((p) => ({
    id: p.id,
    label: p.label,
    blocks: p.blocks.map((b) => ({
      id: b.id,
      title: b.title,
      description: b.description,
    })),
  }));

  const slots: OrganigramSlot[] = [
    ...ORGANIGRAM_ADMIN.people.map((p, i) => personToSlot(p, "admin", i)),
    ...ORGANIGRAM_ACCOUNTING.people.map((p, i) => personToSlot(p, "accounting", i)),
    ...ORGANIGRAM_POLES.flatMap((pole) =>
      pole.blocks.flatMap((block) =>
        block.people.map((p, i) =>
          personToSlot(p, "poles", i, { poleId: pole.id, blockId: block.id }),
        ),
      ),
    ),
    ...ORGANIGRAM_RECEPTION.people.map((p, i) => personToSlot(p, "reception", i)),
    ...ORGANIGRAM_HEALTH.people.map((p, i) => personToSlot(p, "health", i)),
    ...ORGANIGRAM_MAINTENANCE.people.map((p, i) => personToSlot(p, "maintenance", i)),
    ...ORGANIGRAM_PASTORAL.people.map((p, i) => personToSlot(p, "pastoral", i)),
    ...ORGANIGRAM_OGEC.people.map((p, i) => personToSlot(p, "ogec", i)),
    ...ORGANIGRAM_TUTELLE.people.map((p, i) => personToSlot(p, "tutelle", i)),
  ];

  // Labels pôles depuis SCHOOL si besoin de refresh
  for (const pole of poles) {
    if (pole.id === "pole-ecole") pole.label = ESTABLISHMENT_KIND_PRESETS.find((p) => p.kind === "ecole")?.label || pole.label;
    if (pole.id === "pole-college") pole.label = ESTABLISHMENT_KIND_PRESETS.find((p) => p.kind === "college")?.label || pole.label;
    if (pole.id === "pole-lycee") pole.label = ESTABLISHMENT_KIND_PRESETS.find((p) => p.kind === "lycee")?.label || pole.label;
  }

  return { version: 1, sections, poles, slots };
}

/** Seed La Providence uniquement : directions nominatives (PLANTEC, DUMOUCHEL, DONA). */
export function laprovidenceOrganigramConfig(): OrganigramConfig {
  const base = seedOrganigramConfig();
  const directorSlots = ORGANIGRAM_DIRECTORS.map((p, i) =>
    personToSlot(p, "direction", i, {
      establishmentId:
        p.id === "dir-ecole" ? "ecole" : p.id === "dir-college" ? "college" : "lycee",
    }),
  );
  return { ...base, slots: [...directorSlots, ...base.slots] };
}

function asString(v: unknown): string {
  return typeof v === "string" ? v.trim() : "";
}

function asStringArray(v: unknown): string[] {
  if (!Array.isArray(v)) return [];
  return v.map((x) => String(x ?? "").trim()).filter(Boolean);
}

export function parseOrganigramConfig(raw: unknown): OrganigramConfig {
  const seed = seedOrganigramConfig();
  if (!raw || typeof raw !== "object") return seed;
  const o = raw as Record<string, unknown>;

  const sectionIds = new Set(seed.sections.map((s) => s.id));
  const sectionsRaw = Array.isArray(o.sections) ? o.sections : seed.sections;
  const sections: OrganigramSectionMeta[] = [];
  for (const row of sectionsRaw) {
    if (!row || typeof row !== "object") continue;
    const r = row as Record<string, unknown>;
    const id = asString(r.id) as OrganigramSectionId;
    if (!sectionIds.has(id)) continue;
    sections.push({
      id,
      title: asString(r.title) || seed.sections.find((s) => s.id === id)?.title || id,
      description: asString(r.description) || undefined,
      visible: r.visible === false ? false : true,
    });
  }
  for (const s of seed.sections) {
    if (!sections.some((x) => x.id === s.id)) sections.push(s);
  }

  const polesRaw = Array.isArray(o.poles) ? o.poles : seed.poles;
  const poles: OrganigramPoleMeta[] = [];
  for (const row of polesRaw) {
    if (!row || typeof row !== "object") continue;
    const r = row as Record<string, unknown>;
    const id = asString(r.id);
    if (!id) continue;
    const blocksRaw = Array.isArray(r.blocks) ? r.blocks : [];
    const blocks = blocksRaw
      .map((b) => {
        if (!b || typeof b !== "object") return null;
        const bb = b as Record<string, unknown>;
        const bid = asString(bb.id);
        if (!bid) return null;
        return {
          id: bid,
          title: asString(bb.title) || bid,
          description: asString(bb.description) || undefined,
        };
      })
      .filter(Boolean) as OrganigramPoleMeta["blocks"];
    poles.push({
      id,
      label: asString(r.label) || id,
      blocks: blocks.length
        ? blocks
        : seed.poles.find((p) => p.id === id)?.blocks || [],
    });
  }
  if (poles.length === 0) poles.push(...seed.poles);

  const slotsRaw = Array.isArray(o.slots) ? o.slots : seed.slots;
  const slots: OrganigramSlot[] = [];
  for (const row of slotsRaw) {
    if (!row || typeof row !== "object") continue;
    const r = row as Record<string, unknown>;
    const id = asString(r.id);
    const sectionId = asString(r.sectionId) as OrganigramSectionId;
    if (!id || !sectionIds.has(sectionId)) continue;
    const order = Number(r.order);
    slots.push({
      id,
      sectionId,
      poleId: asString(r.poleId) || undefined,
      blockId: asString(r.blockId) || undefined,
      order: Number.isFinite(order) ? order : slots.length,
      email: asString(r.email) || undefined,
      personnelId: asString(r.personnelId) || undefined,
      establishmentId: asString(r.establishmentId) || undefined,
      firstName: asString(r.firstName) || undefined,
      lastName: asString(r.lastName) || undefined,
      role: asString(r.role) || undefined,
      missions: asStringArray(r.missions),
      photoUrl: r.photoUrl == null ? undefined : String(r.photoUrl),
      hidden: Boolean(r.hidden),
    });
  }

  return {
    version: 1,
    sections,
    poles,
    slots: slots.length ? slots : seed.slots,
  };
}

export async function loadOrganigramConfig(): Promise<OrganigramConfig> {
  const hit = await getJson<unknown>(ORGANIGRAMME_CONFIG_KEY);
  if (!hit?.data) {
    const seeded = seedOrganigramConfig();
    await putJson(ORGANIGRAMME_CONFIG_KEY, seeded);
    return seeded;
  }
  return parseOrganigramConfig(hit.data);
}

export async function saveOrganigramConfig(config: OrganigramConfig): Promise<OrganigramConfig> {
  const normalized = parseOrganigramConfig(config);
  await putJson(ORGANIGRAMME_CONFIG_KEY, normalized);
  return normalized;
}

export function canEditOrganigramme(roles: string[]): boolean {
  const n = roles.map((r) => r.toLowerCase());
  return (
    n.some((r) => r === "admin" || r === "org_admin" || r === "master" || r === "platform_admin") ||
    n.some((r) => r.includes("administratif")) ||
    n.some((r) => r.includes("comptabilite"))
  );
}
