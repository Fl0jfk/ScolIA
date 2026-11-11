import { NextResponse } from "next/server";
import { sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user } from "@/db/schema";

/**
 * Ancien raccourci d'activation : il attachait un mot de passe choisi par le visiteur à un compte
 * déjà provisionné, sans preuve de possession de l'e-mail (prise de compte possible en ne
 * connaissant que l'adresse). Il ne pose plus jamais de mot de passe.
 * - adresse inconnue : 404 « Compte inconnu », la page d'inscription continue comme avant ;
 * - compte existant : l'activation passe par le lien reçu par e-mail ou « Mot de passe oublié ».
 */
export async function POST(request: Request) {
  if (!isDatabaseConfigured()) {
    return NextResponse.json({ error: "Auth indisponible." }, { status: 503 });
  }
  let email = "";
  try {
    const body = (await request.json()) as { email?: string };
    email = body.email?.trim().toLowerCase() ?? "";
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  if (!email) {
    return NextResponse.json({ error: "E-mail requis." }, { status: 400 });
  }
  const [row] = await getDb()
    .select({ id: user.id })
    .from(user)
    .where(sql`lower(${user.email}) = ${email}`)
    .limit(1);
  if (!row) {
    return NextResponse.json(
      { error: "Compte inconnu. Utilisez « Créer un compte » pour une nouvelle adresse." },
      { status: 404 },
    );
  }
  return NextResponse.json(
    {
      error:
        "Ce compte existe déjà. Pour l'activer, utilisez le lien reçu par e-mail ou « Mot de passe oublié » sur la page de connexion.",
    },
    { status: 409 },
  );
}
