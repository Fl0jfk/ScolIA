import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import {
  registerPortesOuvertesVisitor,
  updatePortesOuvertesVisitor,
} from "@/app/lib/portes-ouvertes-mail";
import {
  countRegistrationsBySlot,
  listPortesOuvertesRegistrations,
} from "@/app/lib/portes-ouvertes-storage";
import {
  classesForPortesOuvertesCycle,
  isPortesOuvertesRegistrationUpcoming,
  PORTES_OUVERTES_CYCLE_LABELS,
  PORTES_OUVERTES_CYCLES,
} from "@/app/lib/portes-ouvertes-types";

const RegisterSchema = z.object({
  slotId: z.string().min(1),
  firstName: z.string().min(1).max(80),
  lastName: z.string().min(1).max(80),
  email: z.string().email().max(200),
  phone: z.string().min(6).max(40),
  cycle: z.enum(["ecole", "college", "lycee"]),
  classeSouhaitee: z.string().min(1).max(40),
});

const UpdateSchema = z.object({
  id: z.string().min(1),
  slotId: z.string().min(1).optional(),
  firstName: z.string().min(1).max(80).optional(),
  lastName: z.string().min(1).max(80).optional(),
  email: z.string().email().max(200).optional(),
  phone: z.string().min(6).max(40).optional(),
  cycle: z.enum(["ecole", "college", "lycee"]).optional(),
  classeSouhaitee: z.string().min(1).max(40).optional(),
});

function actorFromGate(gate: { ctx: { user: { id: string; firstName?: string; lastName?: string; name?: string } } }) {
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
  const po = toolbox.tools["portes-ouvertes"];
  const registrations = await listPortesOuvertesRegistrations();
  const counts = countRegistrationsBySlot(registrations);
  const now = Date.now();

  const slots = po.slots.map((s) => ({
    ...s,
    registeredCount: counts[s.id] || 0,
    remaining:
      typeof s.maxPlaces === "number" ? Math.max(0, s.maxPlaces - (counts[s.id] || 0)) : null,
    isPast: Date.parse(s.endAt) <= now,
  }));

  const classesByCycle = Object.fromEntries(
    PORTES_OUVERTES_CYCLES.map((c) => [c, classesForPortesOuvertesCycle(c)]),
  );

  const enriched = registrations
    .map((r) => {
      const fromConfig = po.slots.find((s) => s.id === r.slotId);
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
    title: po.title,
    address: po.address,
    mapsUrl: po.mapsUrl || null,
    publicEnabled: po.enabled,
    slots,
    registrations: enriched,
    cycleLabels: PORTES_OUVERTES_CYCLE_LABELS,
    classesByCycle,
  });
}

export async function POST(req: Request) {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;

  const toolbox = await getToolboxConfig();
  const po = toolbox.tools["portes-ouvertes"];

  if (po.slots.length === 0) {
    return NextResponse.json(
      { error: "Aucun créneau configuré. Configurez les portes ouvertes dans Événements." },
      { status: 400 },
    );
  }

  const parsed = RegisterSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json(
      { error: "Nom, prénom, e-mail, téléphone, cycle, classe et créneau sont requis." },
      { status: 400 },
    );
  }

  const body = parsed.data;
  const allowedClasses = classesForPortesOuvertesCycle(body.cycle);
  if (!allowedClasses.includes(body.classeSouhaitee)) {
    return NextResponse.json({ error: "Classe invalide pour ce cycle." }, { status: 400 });
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

  const toolbox = await getToolboxConfig();
  const po = toolbox.tools["portes-ouvertes"];

  const parsed = UpdateSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données de modification invalides." }, { status: 400 });
  }

  const body = parsed.data;
  if (body.cycle && body.classeSouhaitee) {
    const allowed = classesForPortesOuvertesCycle(body.cycle);
    if (!allowed.includes(body.classeSouhaitee)) {
      return NextResponse.json({ error: "Classe invalide pour ce cycle." }, { status: 400 });
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
