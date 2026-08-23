import { NextResponse } from "next/server";
import { requirePlatformMaster } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { isRegistryWritable } from "@/app/lib/tenant-registry";
import { provisionSignupRequestById } from "@/app/lib/platform-signup-provision";

type Ctx = { params: Promise<{ id: string }> };

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requirePlatformMaster();
  if (!gate.ok) return gate.response;

  if (!isRegistryWritable()) {
    return NextResponse.json(
      { error: "Registry non accessible en écriture (REGISTRY_BUCKET)." },
      { status: 400 },
    );
  }

  const { id } = await ctx.params;
  let body: {
    slug?: string;
    hostname?: string;
    dataBucket?: string;
    publishableKey?: string;
    secretKey?: string;
  };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  if (!body.publishableKey?.trim() || !body.secretKey?.trim()) {
    return NextResponse.json({ error: "Clés auth requises." }, { status: 400 });
  }

  try {
    const user = await safeCurrentUser();
    const result = await provisionSignupRequestById(
      id,
      {
        slug: body.slug,
        hostname: body.hostname,
        dataBucket: body.dataBucket,
        publishableKey: body.publishableKey,
        secretKey: body.secretKey,
      },
      user?.id,
    );
    return NextResponse.json({
      ok: true,
      request: result.request,
      tenant: { slug: result.tenant.slug, appUrl: result.tenant.appUrl },
    });
  } catch (e) {
    const msg = e instanceof Error ? e.message : "Provisioning impossible";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
