import type { Establishment } from "@/app/lib/app-config-schemas";
import type { RequestsRoutingConfig } from "@/app/lib/app-config-schemas";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import type {
  OrganigramConfig,
  OrganigramSectionId,
  OrganigramSectionMeta,
  OrganigramSlot,
} from "@/app/lib/organigramme-config";
import type { OrganigramBlock, OrganigramPerson, OrganigramPole } from "@/app/lib/organigramme";
import type { PersonnelIndexEntry, PersonnelRecord } from "@/app/lib/personnel-types";

export type OrganigramView = {
  directors: OrganigramPerson[];
  admin: OrganigramBlock;
  accounting: OrganigramBlock;
  poles: OrganigramPole[];
  reception: OrganigramBlock;
  health: OrganigramBlock;
  maintenance: OrganigramBlock;
  pastoral: OrganigramBlock;
  ogec: OrganigramBlock;
  tutelle: OrganigramBlock;
  sections: OrganigramSectionMeta[];
};

type ResolveCtx = {
  personnelIndex?: PersonnelIndexEntry[];
  personnelById?: Map<string, PersonnelRecord>;
  establishments?: Establishment[];
  routing?: RequestsRoutingConfig | null;
};

function splitName(displayName: string): { firstName?: string; lastName?: string } {
  const parts = displayName.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return {};
  if (parts.length === 1) return { firstName: parts[0] };
  return { firstName: parts[0], lastName: parts.slice(1).join(" ") };
}

function resolveSlotPerson(slot: OrganigramSlot, ctx: ResolveCtx): OrganigramPerson {
  let firstName = slot.firstName;
  let lastName = slot.lastName;
  let role = slot.role || "À préciser";
  let email = slot.email;
  let photoUrl = slot.photoUrl ?? "";
  let missions = [...(slot.missions || [])];

  if (slot.personnelId && ctx.personnelById?.has(slot.personnelId)) {
    const rec = ctx.personnelById.get(slot.personnelId)!;
    firstName = firstName || rec.firstName;
    lastName = lastName || rec.lastName;
    email = email || rec.email;
    role = slot.role || rec.jobTitle || role;
  } else if (email && ctx.personnelIndex?.length) {
    const hit = ctx.personnelIndex.find(
      (e) => e.email.trim().toLowerCase() === email!.trim().toLowerCase(),
    );
    if (hit) {
      if (!firstName && !lastName) {
        const n = splitName(hit.displayName || "");
        firstName = n.firstName;
        lastName = n.lastName;
      }
    }
  }

  if (slot.establishmentId && ctx.establishments?.length) {
    const est =
      ctx.establishments.find((e) => e.id === slot.establishmentId) ||
      ctx.establishments.find(
        (e) =>
          e.kind === slot.establishmentId ||
          e.label.toLowerCase().includes(String(slot.establishmentId)),
      );
    if (est) {
      if ((!firstName && !lastName) && est.directorName) {
        const n = splitName(est.directorName);
        firstName = n.firstName;
        lastName = n.lastName;
      }
      email = email || est.directorEmail || undefined;
      if (!slot.role && est.label) {
        role = `Direction — ${est.label}`;
      }
    }
  }

  return {
    id: slot.id,
    firstName,
    lastName,
    role,
    email,
    photoUrl: photoUrl || "",
    missions,
  };
}

function sectionMeta(config: OrganigramConfig, id: OrganigramSectionId): OrganigramSectionMeta {
  return (
    config.sections.find((s) => s.id === id) || {
      id,
      title: id,
      visible: true,
    }
  );
}

function blockFromSlots(
  config: OrganigramConfig,
  sectionId: OrganigramSectionId,
  ctx: ResolveCtx,
  blockId?: string,
): OrganigramBlock {
  const meta = sectionMeta(config, sectionId);
  const people = config.slots
    .filter(
      (s) =>
        s.sectionId === sectionId &&
        !s.hidden &&
        (blockId ? s.blockId === blockId : !s.blockId || sectionId !== "poles"),
    )
    .sort((a, b) => a.order - b.order || a.id.localeCompare(b.id))
    .map((s) => resolveSlotPerson(s, ctx));

  return {
    id: blockId || sectionId,
    title: meta.title,
    description: meta.description,
    people,
  };
}

function mergeDirectionSlotsFromEstablishments(
  config: OrganigramConfig,
  establishments: Establishment[],
): OrganigramConfig {
  const active = getActiveEstablishments(establishments);
  if (active.length === 0) return config;
  const slots = [...config.slots];
  for (const est of active) {
    const existing = slots.find(
      (s) =>
        s.sectionId === "direction" &&
        !s.hidden &&
        (s.establishmentId === est.id || s.establishmentId === est.kind),
    );
    if (existing) continue;
    const n = splitName(est.directorName || "");
    slots.push({
      id: `dir-${est.id}`,
      sectionId: "direction",
      order: slots.filter((s) => s.sectionId === "direction").length,
      establishmentId: est.id,
      firstName: n.firstName,
      lastName: n.lastName,
      email: est.directorEmail,
      role: `Direction — ${est.label}`,
      missions: [],
    });
  }
  return { ...config, slots };
}

export function buildOrganigramView(config: OrganigramConfig, ctx: ResolveCtx = {}): OrganigramView {
  const merged = mergeDirectionSlotsFromEstablishments(config, ctx.establishments || []);
  const directors = merged.slots
    .filter((s) => s.sectionId === "direction" && !s.hidden)
    .sort((a, b) => a.order - b.order)
    .map((s) => resolveSlotPerson(s, ctx));

  const poles: OrganigramPole[] = config.poles.map((pole) => ({
    id: pole.id,
    label: pole.label,
    blocks: pole.blocks.map((block) => {
      const people = config.slots
        .filter(
          (s) =>
            s.sectionId === "poles" &&
            !s.hidden &&
            s.poleId === pole.id &&
            s.blockId === block.id,
        )
        .sort((a, b) => a.order - b.order)
        .map((s) => resolveSlotPerson(s, ctx));
      return {
        id: block.id,
        title: block.title,
        description: block.description,
        people,
      };
    }),
  }));

  return {
    directors,
    admin: blockFromSlots(config, "admin", ctx),
    accounting: blockFromSlots(config, "accounting", ctx),
    poles,
    reception: blockFromSlots(config, "reception", ctx),
    health: blockFromSlots(config, "health", ctx),
    maintenance: blockFromSlots(config, "maintenance", ctx),
    pastoral: blockFromSlots(config, "pastoral", ctx),
    ogec: blockFromSlots(config, "ogec", ctx),
    tutelle: blockFromSlots(config, "tutelle", ctx),
    sections: config.sections,
  };
}

/** Mappe une tâche / branche tickets vers une section organigramme. */
function suggestSectionFromTaskId(taskId: string): {
  sectionId: OrganigramSectionId;
  poleId?: string;
  blockId?: string;
} | null {
  const t = taskId.toLowerCase();
  if (t.includes("direction_ecole")) return { sectionId: "direction", poleId: undefined };
  if (t.includes("direction_college")) return { sectionId: "direction" };
  if (t.includes("direction_lycee")) return { sectionId: "direction" };
  if (t.startsWith("admin_")) return { sectionId: "admin" };
  if (t.includes("comptab")) return { sectionId: "accounting" };
  if (t.includes("accueil")) return { sectionId: "reception" };
  if (t.includes("maintenance")) return { sectionId: "maintenance" };
  if (t.includes("cpe") || t.includes("vie_scolaire")) {
    if (t.includes("ecole") || t.includes("école")) {
      return { sectionId: "poles", poleId: "pole-ecole", blockId: "vs-ecole" };
    }
    if (t.includes("college") || t.includes("collège") || t.includes("3e") || t.includes("6e")) {
      return { sectionId: "poles", poleId: "pole-college", blockId: "vs-college" };
    }
    if (t.includes("lycee") || t.includes("lycée")) {
      return { sectionId: "poles", poleId: "pole-lycee", blockId: "vs-lycee" };
    }
    return { sectionId: "poles", poleId: "pole-college", blockId: "vs-college" };
  }
  return null;
}

export function suggestSlotsFromRouting(
  config: OrganigramConfig,
  routing: RequestsRoutingConfig,
): OrganigramSlot[] {
  const existingEmails = new Set(
    config.slots.map((s) => (s.email || "").trim().toLowerCase()).filter(Boolean),
  );
  const out: OrganigramSlot[] = [];
  let i = 0;
  for (const a of routing.assignments || []) {
    const email = (a.email || "").trim().toLowerCase();
    if (!email || existingEmails.has(email)) continue;
    const hint = suggestSectionFromTaskId(a.taskId || "");
    if (!hint) continue;
    const name = splitName(a.personName || email.split("@")[0] || "");
    const id = `suggest-${hint.sectionId}-${email.replace(/[^a-z0-9]/g, "")}-${i++}`;
    out.push({
      id,
      sectionId: hint.sectionId,
      poleId: hint.poleId,
      blockId: hint.blockId,
      order: 100 + i,
      email,
      firstName: name.firstName,
      lastName: name.lastName,
      role: a.taskId || "À préciser",
      missions: [],
      photoUrl: "",
    });
    existingEmails.add(email);
  }
  return out;
}

export function suggestSlotsFromPersonnel(
  config: OrganigramConfig,
  index: PersonnelIndexEntry[],
): OrganigramSlot[] {
  const existingEmails = new Set(
    config.slots.map((s) => (s.email || "").trim().toLowerCase()).filter(Boolean),
  );
  const out: OrganigramSlot[] = [];
  let i = 0;
  for (const e of index) {
    if (e.active === false) continue;
    const email = (e.email || "").trim().toLowerCase();
    if (!email || existingEmails.has(email)) continue;
    const sectionId: OrganigramSectionId =
      e.category === "comptabilite"
        ? "accounting"
        : e.category === "maintenance"
          ? "maintenance"
          : e.category === "surveillant" || e.category === "cpe"
            ? "poles"
            : "admin";
    const name = splitName(e.displayName || email.split("@")[0] || "");
    out.push({
      id: `suggest-rh-${e.id}-${i++}`,
      sectionId,
      poleId: sectionId === "poles" ? "pole-college" : undefined,
      blockId: sectionId === "poles" ? "vs-college" : undefined,
      order: 200 + i,
      email,
      personnelId: e.id,
      firstName: name.firstName,
      lastName: name.lastName,
      role: e.category,
      missions: [],
      photoUrl: "",
    });
    existingEmails.add(email);
  }
  return out;
}
