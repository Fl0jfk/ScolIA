import { NextResponse } from "next/server";
import { z } from "zod";
import { loadAppConfig } from "@/app/lib/app-config";
import { requireModule } from "@/app/lib/intranet-auth";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import {
  registerPortesOuvertesVisitor,
  updatePortesOuvertesVisitor,
} from "@/app/lib/portes-ouvertes-mail";
import {
  buildPortesOuvertesToolPayload,
  countRegistrationsBySlot,
  listPortesOuvertesRegistrations,
  listPortesOuvertesStaff,
  markPortesOuvertesVisited,
} from "@/app/lib/portes-ouvertes-db";
import {
  classesForPortesOuvertesCycle,
  cyclesFromActiveEstablishments,
  isPortesOuvertesRegistrationUpcoming,
  PORTES_OUVERTES_CYCLE_LABELS,
  PORTES_OUVERTES_CYCLES,
  type PortesOuvertesCycle,
} from "@/app/lib/portes-ouvertes-types";

const RegisterSchema = z.object({
  slotId: z.string().min(1),
  firstName: z.string().min(1).max(80),
  lastName: z.string().min(1).max(80),
  email: z.string().email().max(200),
  phone: z.string().min(6).max(40),
  cycle: z.enum(["ecole", "college", "lycee"]),
  classeSouhaitee: z.string().min(1).max(80),
});

const UpdateSchema = z.object({
  id: z.string().min(1),
  slotId: z.string().min(1).optional(),
  firstName: z.string().min(1).max(80).optional(),
  lastName: z.string().min(1).max(80).optional(),
  email: z.string().email().max(200).optional(),
  phone: z.string().min(6).max(40).optional(),
  cycle: z.enum(["ecole", "college", "lycee"]).optional(),
  classeSouhaitee: z.string().min(1).max(80).optional(),
});

const VisitSchema = z.object({
  id: z.string().min(1),
  visited: z.boolean(),
});

function actorFromGate(gate: {
  ctx: { user: { id: string; firstName?: string; lastName?: string; name?: string } };
}) {
  const name =
    [gate.ctx.user.firstName, gate.ctx.user.lastName].filter(Boolean).join(" ") ||
    gate.ctx.user.name ||
    "Accueil";
  return { userId: gate.ctx.user.id, name };
}

export async function GET() {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;

  const toolbox = await getToolboxConfig();
  const [payload, registrations, staff, bundle] = await Promise.all([
    buildPortesOuvertesToolPayload(),
    listPortesOuvertesRegistrations(),
    listPortesOuvertesStaff(),
    loadAppConfig(),
  ]);
  const counts = countRegistrationsBySlot(registrations);
  const now = Date.now();
  const availableCycles = cyclesFromActiveEstablishments(bundle.establishments);

  const cycleLabels: Partial<Record<PortesOuvertesCycle, string>> = {
    ...PORTES_OUVERTES_CYCLE_LABELS,
  };
  for (const e of bundle.establishments) {
    if (e.kind === "ecole" || e.kind === "college" || e.kind === "lycee") {
      cycleLabels[e.kind] = e.label || cycleLabels[e.kind];
    }
  }

  const slots = payload.slots.map((s) => {
    const registeredCount = counts[s.id] || 0;
    const remaining =
      typeof s.maxPlaces === "number" ? Math.max(0, s.maxPlaces - registeredCount) : null;
    return {
      ...s,
      registeredCount,
      remaining,
      remainingByCycle: Object.fromEntries(
        PORTES_OUVERTES_CYCLES.map((c) => [
          c,
          !s.cycle || s.cycle === c ? remaining : null,
        ]),
      ) as Record<PortesOuvertesCycle, number | null>,
      registeredByCycle: Object.fromEntries(
        PORTES_OUVERTES_CYCLES.map((c) => [
          c,
          !s.cycle || s.cycle === c ? registeredCount : 0,
        ]),
      ) as Record<PortesOuvertesCycle, number>,
      isPast: Date.parse(s.endAt) <= now,
    };
  });

  const classesByCycle = Object.fromEntries(
    availableCycles.map((c) => [c, classesForPortesOuvertesCycle(c)]),
  ) as Partial<Record<PortesOuvertesCycle, string[]>>;

  const enriched = registrations
    .map((r) => {
      const fromConfig = payload.slots.find((s) => s.id === r.slotId);
      const withSnap = {
        ...r,
        slotLabel: r.slotLabel || fromConfig?.label,
        slotStartAt: r.slotStartAt || fromConfig?.startAt,
        slotEndAt: r.slotEndAt || fromConfig?.endAt,
      };
      return {
        ...withSnap,
        upcoming: isPortesOuvertesRegistrationUpcoming(withSnap, now),
      };
    })
    .sort((a, b) => {
      const aStart = a.slotStartAt || a.createdAt;
      const bStart = b.slotStartAt || b.createdAt;
      return bStart.localeCompare(aStart);
    });

  return NextResponse.json({
    title: payload.title,
    address: payload.address,
    mapsUrl: payload.mapsUrl || null,
    preinscriptionUrl: payload.preinscriptionUrl || null,
    followUpDelayMinutes: payload.followUpDelayMinutes,
    publicEnabled: toolbox.tools["portes-ouvertes"].enabled,
    slots,
    registrations: enriched,
    staff,
    availableCycles,
    cycleLabels,
    classesByCycle,
  });
}

export async function POST(req: Request) {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;

  const payload = await buildPortesOuvertesToolPayload();
  const po = {
    enabled: true,
    ...payload,
  };

  if (po.slots.length === 0) {
    return NextResponse.json(
      { error: "Aucun créneau configuré. Configurez les portes ouvertes dans Événements." },
      { status: 400 },
    );
  }

  const bodyJson = await req.json().catch(() => null);
  if (bodyJson && typeof bodyJson === "object" && "visited" in (bodyJson as object)) {
    const visitParsed = VisitSchema.safeParse(bodyJson);
    if (!visitParsed.success) {
      return NextResponse.json({ error: "Check-in invalide." }, { status: 400 });
    }
    const entry = await markPortesOuvertesVisited(visitParsed.data.id, visitParsed.data.visited);
    if (!entry) return NextResponse.json({ error: "Inscription introuvable." }, { status: 404 });
    return NextResponse.json({
      success: true,
      entry,
      followUpDelayMinutes: payload.followUpDelayMinutes,
    });
  }

  const parsed = RegisterSchema.safeParse(bodyJson);
  if (!parsed.success) {
    return NextResponse.json(
      { error: "Nom, prénom, e-mail, téléphone, cycle, classe et créneau sont requis." },
      { status: 400 },
    );
  }

  const body = parsed.data;
  const bundle = await loadAppConfig();
  const availableCycles = cyclesFromActiveEstablishments(bundle.establishments);
  if (!availableCycles.includes(body.cycle)) {
    return NextResponse.json(
      { error: "Ce cycle n’est pas proposé pour cet établissement." },
      { status: 400 },
    );
  }

  const result = await registerPortesOuvertesVisitor(po, {
    slotId: body.slotId,
    firstName: body.firstName.trim(),
    lastName: body.lastName.trim(),
    email: body.email.trim().toLowerCase(),
    phone: body.phone.trim(),
    cycle: body.cycle,
    classeSouhaitee: body.classeSouhaitee,
    consent: true,
    source: "accueil",
    recordedBy: actorFromGate(gate),
  });

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: result.status });
  }

  return NextResponse.json({
    success: true,
    registrationId: result.entry.id,
    mailSent: result.mailSent,
    entry: result.entry,
  });
}

export async function PATCH(req: Request) {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;

  const payload = await buildPortesOuvertesToolPayload();
  const po = { enabled: true, ...payload };

  const parsed = UpdateSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données de modification invalides." }, { status: 400 });
  }

  const body = parsed.data;
  if (body.cycle) {
    const bundle = await loadAppConfig();
    const availableCycles = cyclesFromActiveEstablishments(bundle.establishments);
    if (!availableCycles.includes(body.cycle)) {
      return NextResponse.json(
        { error: "Ce cycle n’est pas proposé pour cet établissement." },
        { status: 400 },
      );
    }
  }

  const result = await updatePortesOuvertesVisitor(po, {
    id: body.id,
    slotId: body.slotId,
    firstName: body.firstName,
    lastName: body.lastName,
    email: body.email,
    phone: body.phone,
    cycle: body.cycle,
    classeSouhaitee: body.classeSouhaitee,
    actor: actorFromGate(gate),
  });

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: result.status });
  }

  return NextResponse.json({
    success: true,
    mailSent: result.mailSent,
    entry: result.entry,
  });
}
