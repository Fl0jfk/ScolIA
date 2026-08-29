import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  loadUserSignatureBytes,
  parseSignaturePngBase64,
  saveUserSignature,
} from "@/app/lib/user-signature-store";

export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const bytes = await loadUserSignatureBytes(gate.ctx.userId);
    return NextResponse.json({ hasSignature: Boolean(bytes?.length) });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const body = await req.json();
    const png = parseSignaturePngBase64(String(body.signaturePngBase64 ?? ""));
    if (!png) {
      return NextResponse.json({ error: "Image PNG invalide." }, { status: 400 });
    }

    await saveUserSignature(gate.ctx.userId, png);
    return NextResponse.json({ success: true });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
