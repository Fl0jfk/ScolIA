import { NextResponse } from "next/server";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { requireAuth } from "@/app/lib/intranet-auth";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const members = await listDirectoryMembers();
    const users = members.map((m) => ({
      id: m.externalUserId || m.email,
      name: m.displayName ?? (`${m.firstName ?? ""} ${m.lastName ?? ""}`.trim() || m.email),
      email: m.email,
      avatar: null as string | null,
    }));
    return NextResponse.json(users);
  } catch (error) {
    console.error(error);
    return NextResponse.json({ error: "Impossible de lister les personnels" }, { status: 500 });
  }
}
