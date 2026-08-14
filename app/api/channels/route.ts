import { NextRequest, NextResponse } from "next/server";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { requireAuth } from "@/app/lib/intranet-auth";

const FILE_KEY = "channels/channels.json";
const DEFAULT_CHANNELS = [{ id: "general", name: "Général", type: "public" }];

type ChannelRow = { id?: string; name?: string; type?: string; [key: string]: unknown };

function channelsFromHit(data: unknown): ChannelRow[] {
  return Array.isArray(data) ? (data as ChannelRow[]) : [...DEFAULT_CHANNELS];
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const hit = await getJson<unknown[]>( FILE_KEY);
    const data = Array.isArray(hit?.data) ? hit.data : DEFAULT_CHANNELS;
    return NextResponse.json(data);
  } catch (error) {
    console.error("Erreur GET Channels:", error);
    return NextResponse.json(DEFAULT_CHANNELS);
  }
}

export async function POST(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const hit = await getJson<unknown[]>( FILE_KEY);
    const channels = channelsFromHit(hit?.data);
    const newChan = {
      id: String(body.id || body.name || "").trim().toLowerCase().replace(/\s+/g, "-"),
      name: String(body.name || "Nouveau canal"),
      type: body.type || "public",
      members: body.members || [],
    };
    channels.push(newChan);
    await putJson(FILE_KEY, channels);
    return NextResponse.json(newChan);
  } catch (error) {
    console.error("Erreur POST Channels:", error);
    return NextResponse.json({ error: "Erreur création canal" }, { status: 500 });
  }
}

export async function PATCH(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const hit = await getJson<unknown[]>( FILE_KEY);
    const channels = channelsFromHit(hit?.data);
    const idx = channels.findIndex((c) => c.id === body.id);
    if (idx === -1) return NextResponse.json({ error: "Canal introuvable" }, { status: 404 });
    channels[idx] = { ...channels[idx], ...body };
    await putJson(FILE_KEY, channels);
    return NextResponse.json(channels[idx]);
  } catch (error) {
    return NextResponse.json({ error: "Erreur modification" }, { status: 500 });
  }
}

export async function DELETE(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const { searchParams } = new URL(req.url);
    const id = searchParams.get("id");
    const hit = await getJson<unknown[]>( FILE_KEY);
    let channels = channelsFromHit(hit?.data);
    channels = channels.filter((c) => c.id !== id);
    await putJson(FILE_KEY, channels);
    return NextResponse.json({ success: true });
  } catch (error) {
    return NextResponse.json({ error: "Erreur suppression" }, { status: 500 });
  }
}
