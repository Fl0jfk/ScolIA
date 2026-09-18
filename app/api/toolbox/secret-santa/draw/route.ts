import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { drawSecretSanta } from "@/app/lib/secret-santa";
import { getToolboxConfig } from "@/app/lib/toolbox-config";

export async function POST(req: Request) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const toolbox = await getToolboxConfig();
    const santa = toolbox.tools["secret-santa"];
    if (!santa.enabled) {
      return NextResponse.json({ error: "Secret Santa non activé." }, { status: 403 });
    }

    const body = await req.json();
    const names = Array.isArray(body.names)
      ? body.names.map((n: unknown) => String(n).trim()).filter(Boolean)
      : santa.participantNames;

    const pairs = drawSecretSanta(names);
    return NextResponse.json({
      success: true,
      pairs,
      budgetHint: santa.budgetHint,
      title: santa.title,
    });
  } catch (e) {
    return NextResponse.json({ error: e instanceof Error ? e.message : String(e) }, { status: 400 });
  }
}
