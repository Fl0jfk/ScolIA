import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import { registerPortesOuvertesVisitor } from "@/app/lib/portes-ouvertes-mail";
import {
  cyclesFromActiveEstablishments,
  isPortesOuvertesCycle,
} from "@/app/lib/portes-ouvertes-types";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const portesOuvertesLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 10,
});

export async function POST(req: Request) {
  try {
    if (!(await portesOuvertesLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const toolbox = await getToolboxConfig();
    const po = toolbox.tools["portes-ouvertes"];
    if (!po.enabled) {
      return NextResponse.json({ error: "Les portes ouvertes ne sont pas activées." }, { status: 403 });
    }

    const body = await req.json();
    const honeypot = String(body.website || body.company || "").trim();
    if (honeypot) {
      return NextResponse.json({ success: true });
    }
    const slotId = String(body.slotId || "").trim();
    const firstName = String(body.firstName || "").trim();
    const lastName = String(body.lastName || "").trim();
    const email = String(body.email || "").trim().toLowerCase();
    const phone = String(body.phone || "").trim() || undefined;
    const childFirstName = String(body.childFirstName || "").trim();
    const childLastName = String(body.childLastName || "").trim();
    const classeSouhaitee = String(body.classeSouhaitee || "").trim();
    const cycleRaw = String(body.cycle || "").trim();
    const consent = body.consent === true;

    if (!slotId || !firstName || !lastName || !email) {
      return NextResponse.json({ error: "Créneau, nom, prénom et e-mail requis." }, { status: 400 });
    }
    if (!isPortesOuvertesCycle(cycleRaw)) {
      return NextResponse.json({ error: "Veuillez sélectionner un cycle (école, collège ou lycée)." }, { status: 400 });
    }
    if (!childFirstName || !childLastName || !classeSouhaitee) {
      return NextResponse.json(
        { error: "Prénom, nom de l’enfant et classe souhaitée sont requis." },
        { status: 400 },
      );
    }
    if (childFirstName.length > 80 || childLastName.length > 80 || classeSouhaitee.length > 80) {
      return NextResponse.json({ error: "Champs enfant trop longs." }, { status: 400 });
    }
    if (!consent) {
      return NextResponse.json({ error: "Veuillez accepter le traitement de vos données." }, { status: 400 });
    }

    const bundle = await loadAppConfig();
    const allowedCycles = cyclesFromActiveEstablishments(bundle.establishments);
    if (!allowedCycles.includes(cycleRaw)) {
      return NextResponse.json({ error: "Ce cycle n’est pas proposé pour cet établissement." }, { status: 400 });
    }

    const result = await registerPortesOuvertesVisitor(po, {
      slotId,
      firstName,
      lastName,
      email,
      phone,
      childFirstName,
      childLastName,
      cycle: cycleRaw,
      classeSouhaitee,
      consent,
      source: "public",
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    return NextResponse.json({ success: true, registrationId: result.entry.id });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
