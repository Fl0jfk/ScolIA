import { NextResponse } from "next/server";
import { listPublicPartenariatCards } from "@/app/lib/partenariats-db";

export async function GET() {
  try {
    const data = await listPublicPartenariatCards();
    return NextResponse.json(data);
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
