import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { getToolboxConfig, saveToolboxConfig } from "@/app/lib/toolbox-config";
import {
  addPortesOuvertesStaff,
  buildPortesOuvertesToolPayload,
  deletePortesOuvertesSlot,
  deletePortesOuvertesStaff,
  insertPortesOuvertesSlots,
  listPortesOuvertesRegistrations,
  replacePortesOuvertesSlotsForCycle,
  upsertPortesOuvertesConfig,
  upsertPortesOuvertesSlot,
  countRegistrationsBySlot,
} from "@/app/lib/portes-ouvertes-db";
import { isPortesOuvertesCycle } from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesCycle } from "@/app/lib/portes-ouvertes-types";

const SlotSchema = z.object({
  id: z.string().min(1),
  label: z.string().min(1),
  startAt: z.string().min(1),
  endAt: z.string().min(1),
  maxPlaces: z.number().int().positive().optional(),
  cycle: z.enum(["ecole", "college", "lycee"]),
});

const StaffAddSchema = z.object({
  slotId: z.string().min(1),
  role: z.enum(["ambassadeur", "enseignant", "personnel"]),
  refId: z.string().min(1).max(120),
  displayName: z.string().min(1).max(200),
  meta: z.record(z.string(), z.string()).optional(),
});

const PutSchema = z.object({
  title: z.string().min(1).max(200).optional(),
  intro: z.string().max(4000).optional(),
  address: z.string().max(500).optional(),
  mapsUrl: z.string().max(1000).optional().nullable(),
  notifyEmail: z.string().max(200).optional().nullable(),
  preinscriptionUrl: z.string().max(1000).optional().nullable(),
  followUpDelayMinutes: z.number().int().min(5).max(24 * 60).optional(),
  consentLabel: z.string().max(1000).optional(),
  enabled: z.boolean().optional(),
  slotsAppend: z.array(SlotSchema).optional(),
  slotsReplaceCycle: z
    .object({
      cycle: z.enum(["ecole", "college", "lycee"]),
      slots: z.array(SlotSchema),
    })
    .optional(),
  slotUpsert: SlotSchema.optional(),
  slotDeleteId: z.string().min(1).optional(),
  staffAdd: StaffAddSchema.optional(),
  staffDeleteId: z.string().min(1).optional(),
});

async function poAdminResponse() {
  const [toolbox, payload, registrations] = await Promise.all([
    getToolboxConfig(),
    buildPortesOuvertesToolPayload(),
    listPortesOuvertesRegistrations(),
  ]);
  return {
    enabled: toolbox.tools["portes-ouvertes"].enabled,
    ...payload,
    stats: countRegistrationsBySlot(registrations),
    registrationsCount: registrations.length,
  };
}

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  try {
    return NextResponse.json(await poAdminResponse());
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  try {
    const parsed = PutSchema.safeParse(await req.json().catch(() => null));
    if (!parsed.success) {
      return NextResponse.json({ error: "Données invalides." }, { status: 400 });
    }
    const body = parsed.data;

    if (body.enabled !== undefined) {
      const toolbox = await getToolboxConfig();
      await saveToolboxConfig({
        ...toolbox,
        tools: {
          ...toolbox.tools,
          "portes-ouvertes": {
            ...toolbox.tools["portes-ouvertes"],
            enabled: body.enabled,
            slots: [],
          },
        },
      });
    }

    await upsertPortesOuvertesConfig({
      title: body.title,
      intro: body.intro,
      address: body.address,
      mapsUrl: body.mapsUrl === null ? "" : body.mapsUrl,
      notifyEmail: body.notifyEmail === null ? "" : body.notifyEmail,
      preinscriptionUrl: body.preinscriptionUrl === null ? "" : body.preinscriptionUrl,
      followUpDelayMinutes: body.followUpDelayMinutes,
      consentLabel: body.consentLabel,
    });

    if (body.slotDeleteId) {
      await deletePortesOuvertesSlot(body.slotDeleteId);
    }
    if (body.slotUpsert) {
      await upsertPortesOuvertesSlot(body.slotUpsert);
    }
    if (body.slotsAppend?.length) {
      await insertPortesOuvertesSlots(body.slotsAppend);
    }
    if (body.slotsReplaceCycle) {
      const cycle = body.slotsReplaceCycle.cycle as PortesOuvertesCycle;
      if (!isPortesOuvertesCycle(cycle)) {
        return NextResponse.json({ error: "Cycle invalide." }, { status: 400 });
      }
      await replacePortesOuvertesSlotsForCycle(cycle, body.slotsReplaceCycle.slots);
    }
    if (body.staffDeleteId) {
      const ok = await deletePortesOuvertesStaff(body.staffDeleteId);
      if (!ok) return NextResponse.json({ error: "Staffing introuvable." }, { status: 404 });
    }
    if (body.staffAdd) {
      await addPortesOuvertesStaff(body.staffAdd);
    }

    return NextResponse.json({
      success: true,
      ...(await poAdminResponse()),
    });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : String(e) },
      { status: 400 },
    );
  }
}
