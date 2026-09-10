import { headers } from "next/headers";
import { NextResponse } from "next/server";
import { getAuthenticatorName } from "@better-auth/passkey";
import { getBetterAuth } from "@/app/lib/auth-server";
import {
  deletePasskeyForUser,
  listPasskeysForUser,
} from "@/app/lib/passkey-db";
import { consumeRateLimit } from "@/app/lib/rate-limit";
import { writeSecurityAudit } from "@/app/lib/security-audit";

/** Liste / suppression des passkeys du compte connecté. */
export async function GET() {
  const auth = getBetterAuth();
  const session = await auth.api.getSession({ headers: await headers() });
  if (!session?.user?.id) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }
  const rows = await listPasskeysForUser(session.user.id);
  return NextResponse.json({
    passkeys: rows.map((p) => ({
      id: p.id,
      name: p.name || getAuthenticatorName(p.aaguid ?? undefined) || "Passkey téléphone / clé",
      deviceType: p.deviceType,
      backedUp: p.backedUp,
      createdAt: p.createdAt?.toISOString() ?? null,
    })),
  });
}

export async function DELETE(req: Request) {
  const auth = getBetterAuth();
  const session = await auth.api.getSession({ headers: req.headers });
  if (!session?.user?.id) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const fwd = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() || "unknown";
  const rate = await consumeRateLimit({
    key: `passkey-del:${session.user.id}:${fwd}`,
    limit: 20,
    windowMs: 15 * 60 * 1000,
  });
  if (!rate.ok) {
    return NextResponse.json({ error: "Trop de tentatives." }, { status: 429 });
  }

  let id = "";
  try {
    const body = (await req.json()) as { id?: string };
    id = String(body.id ?? "").trim();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  if (!id) {
    return NextResponse.json({ error: "Identifiant passkey manquant." }, { status: 400 });
  }

  const ok = await deletePasskeyForUser(session.user.id, id);
  if (!ok) {
    return NextResponse.json({ error: "Passkey introuvable." }, { status: 404 });
  }

  await writeSecurityAudit({
    userId: session.user.id,
    action: "passkey_deleted",
    req,
    metadata: { passkeyId: id },
  });

  return NextResponse.json({ ok: true });
}
