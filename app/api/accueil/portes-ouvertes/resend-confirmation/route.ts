import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  buildPortesOuvertesToolPayload,
  listPortesOuvertesRegistrations,
} from "@/app/lib/portes-ouvertes-db";
import { resendPortesOuvertesVisitorConfirmation } from "@/app/lib/portes-ouvertes-mail";

const BodySchema = z.object({
  /** Nombre des dernières inscriptions à renvoyer (défaut 2, max 10). */
  limit: z.number().int().min(1).max(10).optional(),
  /** Sinon, ids ciblés. */
  ids: z.array(z.string().min(1)).max(10).optional(),
});

/**
 * Renvoi manuel des mails de confirmation portes ouvertes.
 * POST { limit?: 2 } ou { ids: ["…"] }
 * Auth : session module, ou header `x-scola-support-secret` (= OCR_WORKER_SECRET / TRAVEL_EMAIL_INGEST_SECRET).
 */
export async function POST(req: Request) {
  const supportSecret = req.headers.get("x-scola-support-secret")?.trim() || "";
  const expected =
    process.env.OCR_WORKER_SECRET?.trim() ||
    process.env.TRAVEL_EMAIL_INGEST_SECRET?.trim() ||
    "";
  const supportOk = Boolean(expected && supportSecret && supportSecret === expected);
  if (!supportOk) {
    const gate = await requireModule("accueil-portes-ouvertes");
    if (!gate.ok) return gate.response;
  }

  const parsed = BodySchema.safeParse(await req.json().catch(() => ({})));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  const payload = await buildPortesOuvertesToolPayload();
  const po = { enabled: true, ...payload };
  const all = await listPortesOuvertesRegistrations();
  const sorted = [...all].sort((a, b) => b.createdAt.localeCompare(a.createdAt));

  const targets = parsed.data.ids?.length
    ? sorted.filter((r) => parsed.data.ids!.includes(r.id))
    : sorted.slice(0, parsed.data.limit ?? 2);

  if (targets.length === 0) {
    return NextResponse.json({ error: "Aucune inscription trouvée." }, { status: 404 });
  }

  const results: Array<{ id: string; email: string; ok: boolean }> = [];
  for (const entry of targets) {
    const slotFromConfig = po.slots.find((s) => s.id === entry.slotId);
    const startAt = entry.slotStartAt || slotFromConfig?.startAt;
    const endAt = entry.slotEndAt || slotFromConfig?.endAt;
    if (!startAt || !endAt) {
      results.push({ id: entry.id, email: entry.email, ok: false });
      continue;
    }
    const slot = {
      id: entry.slotId,
      label: entry.slotLabel || slotFromConfig?.label || "Créneau",
      startAt,
      endAt,
      cycle: entry.cycle || slotFromConfig?.cycle,
      maxPlaces: slotFromConfig?.maxPlaces,
    };
    const ok = await resendPortesOuvertesVisitorConfirmation({ po, entry, slot });
    results.push({ id: entry.id, email: entry.email, ok });
  }

  return NextResponse.json({
    success: true,
    sent: results.filter((r) => r.ok).length,
    failed: results.filter((r) => !r.ok).length,
    results,
  });
}
