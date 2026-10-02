import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  deleteInvitationEligible,
  listInvitationEligible,
  replaceInvitationEligibleBatch,
} from "@/app/lib/invitation-db";
import {
  INVITATION_DIPLOMAS,
  isInvitationDiploma,
  parseInvitationBirthDate,
} from "@/app/lib/invitation-types";

type Ctx = { params: Promise<{ pageId: string }> };

const ImportSchema = z.object({
  mode: z.enum(["append", "replace"]).default("append"),
  text: z.string().max(200_000).optional(),
  rows: z
    .array(
      z.object({
        eleveFirstName: z.string().min(1).max(80),
        eleveLastName: z.string().min(1).max(80),
        birthDate: z.string().max(32).nullable().optional(),
        diploma: z.enum(INVITATION_DIPLOMAS).nullable().optional(),
      }),
    )
    .max(5000)
    .optional(),
});

/**
 * Parse collé CSV / lignes :
 * - Prénom;Nom;JJ/MM/AAAA
 * - Prénom;Nom;bac;JJ/MM/AAAA
 * - Prénom;Nom;JJ/MM/AAAA;bac
 * - Prénom Nom JJ/MM/AAAA
 */
export function parseEligiblePaste(text: string): {
  eleveFirstName: string;
  eleveLastName: string;
  birthDate: string | null;
  diploma: "bac" | "brevet" | null;
}[] {
  const out: {
    eleveFirstName: string;
    eleveLastName: string;
    birthDate: string | null;
    diploma: "bac" | "brevet" | null;
  }[] = [];

  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || /^prenom/i.test(line) || /^nom/i.test(line)) continue;

    const parts = line.split(/[;\t]/).map((p) => p.trim()).filter(Boolean);
    let eleveFirstName = "";
    let eleveLastName = "";
    let birthDate: string | null = null;
    let diploma: "bac" | "brevet" | null = null;

    const consumeToken = (token: string) => {
      const dip = token.toLowerCase();
      if (isInvitationDiploma(dip)) {
        diploma = dip;
        return;
      }
      const bd = parseInvitationBirthDate(token);
      if (bd) {
        birthDate = bd;
        return;
      }
    };

    if (parts.length >= 2) {
      eleveFirstName = parts[0];
      eleveLastName = parts[1];
      for (const extra of parts.slice(2)) consumeToken(extra);
    } else {
      const tokens = line.split(/\s+/).filter(Boolean);
      if (tokens.length < 2) continue;
      // Derniers tokens peuvent être date / diplôme
      const rest = [...tokens];
      while (rest.length > 2) {
        const last = rest[rest.length - 1];
        const before: "bac" | "brevet" | null = diploma;
        const beforeBd: string | null = birthDate;
        consumeToken(last);
        if (diploma !== before || birthDate !== beforeBd) {
          rest.pop();
        } else {
          break;
        }
      }
      if (rest.length < 2) continue;
      eleveFirstName = rest[0];
      eleveLastName = rest.slice(1).join(" ");
    }

    if (!eleveFirstName || !eleveLastName) continue;
    out.push({ eleveFirstName, eleveLastName, birthDate, diploma });
  }
  return out;
}

export async function GET(_req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const eligible = await listInvitationEligible(pageId);
    return NextResponse.json({ eligible });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const body = ImportSchema.parse(await req.json());
    const rows =
      body.rows && body.rows.length > 0
        ? body.rows.map((r) => ({
            ...r,
            birthDate: r.birthDate ? parseInvitationBirthDate(r.birthDate) : null,
          }))
        : parseEligiblePaste(body.text || "");
    if (rows.length === 0) {
      return NextResponse.json({ error: "Aucun élève à importer." }, { status: 400 });
    }
    const result = await replaceInvitationEligibleBatch(pageId, rows, body.mode);
    const eligible = await listInvitationEligible(pageId);
    return NextResponse.json({ ...result, eligible });
  } catch (e) {
    if (e instanceof z.ZodError) {
      return NextResponse.json(
        { error: e.issues[0]?.message || "Données invalides." },
        { status: 400 },
      );
    }
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function DELETE(req: Request, ctx: Ctx) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const { pageId } = await ctx.params;
    const url = new URL(req.url);
    const eligibleId = url.searchParams.get("id");
    if (!eligibleId) {
      return NextResponse.json({ error: "id requis." }, { status: 400 });
    }
    const ok = await deleteInvitationEligible(pageId, eligibleId);
    if (!ok) return NextResponse.json({ error: "Ligne introuvable." }, { status: 404 });
    return NextResponse.json({ success: true });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
