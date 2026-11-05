/**
 * Parcours « fiches de dialogue 3ᵉ » de bout en bout — Postgres local uniquement.
 * Vérifie les correctifs du 05/11/2026 : ciblage de toutes les divisions (3A…3F),
 * refus des campagnes multi-niveaux, voies 2ⁿᵈᵉ pro / CAP, adresses parents jamais
 * renvoyées en clair par la page publique (références opaques), puis saisie parent.
 *
 * Lancement (PowerShell) :
 *   $env:TEST_DATABASE_URL="postgresql://scola:scola_dev_pwd@127.0.0.1:5432/scolia_migrate"
 *   $env:SCOLIA_TEST_MODE="1"
 *   npx tsx --require ./scripts/stub-server-only.cjs scripts/audit-fiches-dialogue-ciblage-3e.ts
 *
 * Aucun e-mail réel (pas de SMTP, adresses @example.test), aucune connexion prod.
 */
import { randomUUID } from "node:crypto";
import { and, desc, eq } from "drizzle-orm";
import { beginTestDatabase, endTestDatabase } from "@/app/lib/test-database-harness";

let ok = 0;
let ko = 0;
function check(label: string, cond: boolean, detail = "") {
  if (cond) ok += 1;
  else ko += 1;
  console.log(`${cond ? "OK" : "KO"} | ${label}${detail ? ` | ${detail}` : ""}`);
}
const uniq = (p: string) => `${p}-${randomUUID().slice(0, 8)}`;

async function main() {
  const fakeTest = {
    skip(reason: string) {
      throw new Error(reason);
    },
  } as unknown as import("node:test").TestContext;
  if (!beginTestDatabase(fakeTest)) throw new Error("TEST_DATABASE_URL absente ou refusée");

  const { getDb, closeDb } = await import("@/db/index");
  const schema = await import("@/db/schema");
  const wf = await import("@/app/lib/fiches-dialogue-workflow");
  const route = await import("@/app/api/fiches-dialogue/public/route");
  const db = getDb();

  try {
    const [etab] = await db
      .insert(schema.etablissement)
      .values({ slug: uniq("fd-3e"), name: "Collège audit 3e" })
      .returning();
    const etablissementId = etab.id;
    process.env.SCOLA_TEST_ETAB_ID = etablissementId;

    const classes3e = ["3A", "3B", "3C", "3D", "3E", "3F"];
    const P1 = "parent1.fd3e@example.test";
    const P2 = "parent2.fd3e@example.test";
    for (const classe of [...classes3e, "4A", "4E"]) {
      await db.insert(schema.eleve).values({
        etablissementId,
        sourceKey: uniq(`fd3e:${classe}`),
        nom: `NOM${classe}`,
        prenom: `Prenom${classe}`,
        folderName: uniq(`dossier-${classe}`),
        classe,
        dateNaissance: "2011-03-14",
        parent1Email: P1,
        parent2Email: P2,
      });
    }

    // 1. Campagne multi-niveaux (comme l’ancienne campagne collège) : génération refusée.
    const multi = await wf.createFdCampagneFromTemplate({
      etablissementId,
      templateKey: "college_6e",
      label: "Collège tous niveaux",
      anneeLabel: "2026-2027",
      classesCibles: ["6e", "5e", "4e", "3e"],
    });
    let multiErr = "";
    try {
      await wf.generateFdFichesForCampagne(etablissementId, multi.campagne.id);
    } catch (e) {
      multiErr = e instanceof Error ? e.message : String(e);
    }
    check("Campagne 6e+5e+4e+3e refusée à la génération", /plusieurs niveaux/.test(multiErr), multiErr);

    // 2. Campagne 3e, cible « 3e » (niveau) : les 6 divisions, pas la 4e.
    const created = await wf.createFdCampagneFromTemplate({
      etablissementId,
      niveauActuel: "3e",
      starterMode: "famille_dabord",
      label: "Fiches de dialogue — 3e",
      anneeLabel: "2026-2027",
      classesCibles: ["3e"],
      contactPpLabel: "École Directe",
    });
    const campagneId = created.campagne.id;
    const destIds = (created.campagne.catalogue?.destinations ?? []).map((d) => d.id);
    check(
      "Voies 2nde GT, 2nde pro et CAP proposées",
      ["2nde", "2nde_pro", "cap"].every((id) => destIds.includes(id)),
      destIds.join(", "),
    );
    await wf.updateFdCampagne(etablissementId, campagneId, { statut: "active" });
    const gen = await wf.generateFdFichesForCampagne(etablissementId, campagneId);
    const fiches = await db
      .select()
      .from(schema.fdFiche)
      .where(eq(schema.fdFiche.campagneId, campagneId));
    const classesFiches = [...new Set(fiches.map((f) => f.classeActuelle))].sort();
    check(
      "Génération : une fiche par élève de 3A à 3F, aucune en 4e",
      gen.created === 6 && classesFiches.join(",") === classes3e.join(","),
      `créées=${gen.created}, classes=${classesFiches.join(",")}`,
    );

    // 3. Ciblage par classes précises (wizard) : seulement 3A et 3F.
    const partiel = await wf.createFdCampagneFromTemplate({
      etablissementId,
      niveauActuel: "3e",
      starterMode: "famille_dabord",
      label: "3e partiel",
      anneeLabel: "2026-2027",
      classesCibles: ["3A", "3F"],
    });
    const genP = await wf.generateFdFichesForCampagne(etablissementId, partiel.campagne.id);
    check("Ciblage 3A + 3F : 2 fiches", genP.created === 2, `créées=${genP.created}`);
    await wf.updateFdCampagne(etablissementId, partiel.campagne.id, { statut: "cloturee" });

    // 4. Envoi (sans SMTP : compté en échec, aucun mail réel).
    const send = await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId });
    check("Envoi sans SMTP compté en échec", send.sent === 0, `sent=${send.sent}, failed=${send.failed}`);

    // 5. Page publique : identification par nom + prénom + date de naissance.
    const post = async (body: unknown) => {
      const res = await route.POST(
        new Request("http://localhost/api/fiches-dialogue/public", {
          method: "POST",
          headers: { "content-type": "application/json", "x-forwarded-for": `10.0.0.${Math.floor(Math.random() * 200)}` },
          body: JSON.stringify(body),
        }),
      );
      return { status: res.status, json: (await res.json()) as Record<string, unknown> };
    };
    const ident = await post({
      action: "identify",
      nom: "NOM3C",
      prenom: "Prenom3C",
      dateNaissance: "2011-03-14",
    });
    const raw = JSON.stringify(ident.json);
    const match = ident.json.match as
      | { ficheId: string; maskedEmails: Array<{ ref?: string; masked: string; email?: string }> }
      | undefined;
    check("Identification réussie", !!match, `status=${ident.status}`);
    check(
      "Aucune adresse parent en clair dans la réponse",
      !raw.includes(P1) && !raw.includes(P2),
      match?.maskedEmails.map((m) => m.masked).join(", "),
    );
    check(
      "Chaque adresse a une référence opaque",
      !!match && match.maskedEmails.length === 2 && match.maskedEmails.every((m) => /^[a-f0-9]{32}$/.test(m.ref ?? "")),
    );
    if (!match) throw new Error("identification impossible");

    // Ancien format (adresse en clair) refusé.
    const legacy = await post({ action: "request_code", ficheId: match.ficheId, email: P1 });
    check("Demande de code avec adresse en clair refusée (ancien format)", legacy.status === 400, `status=${legacy.status}`);

    // Référence inventée : même réponse neutre, aucun code créé.
    const before = await db.select().from(schema.fdToken).where(eq(schema.fdToken.ficheId, match.ficheId));
    await post({ action: "request_code", ficheId: match.ficheId, emailRef: "0".repeat(32) });
    const afterFake = await db.select().from(schema.fdToken).where(eq(schema.fdToken.ficheId, match.ficheId));
    check("Référence inventée : aucun code créé", afterFake.length === before.length);

    // Vraie référence : code créé pour la bonne adresse.
    const ref1 = match.maskedEmails[0].ref!;
    const reqCode = await post({ action: "request_code", ficheId: match.ficheId, emailRef: ref1 });
    const [tok] = await db
      .select()
      .from(schema.fdToken)
      .where(and(eq(schema.fdToken.ficheId, match.ficheId)))
      .orderBy(desc(schema.fdToken.createdAt))
      .limit(1);
    check(
      "Code envoyé à l’adresse choisie",
      reqCode.status === 200 && !!tok?.secureCode && [P1, P2].includes(tok.email ?? ""),
      `adresse=${tok?.email}`,
    );

    // Code saisi via la référence (sans jamais connaître l’adresse).
    const wrongFiche = fiches.find((f) => f.id !== match.ficheId)!;
    const bad = await post({ action: "resolve_code", ficheId: wrongFiche.id, emailRef: ref1, code: tok.secureCode });
    check("Code refusé sur une autre fiche", bad.status === 404, `status=${bad.status}`);
    const good = await post({ action: "resolve_code", ficheId: match.ficheId, emailRef: ref1, code: tok.secureCode });
    const token = String(good.json.token ?? "");
    check("Code accepté via la référence", good.status === 200 && token.length > 10);
    const typed = await post({ action: "resolve_code", email: tok.email, code: tok.secureCode });
    check("« J’ai déjà un code » avec adresse tapée : toujours possible", typed.status === 200);

    // 6. Ouverture de la fiche et saisie « 2nde professionnelle ».
    const ctxRes = await route.GET(new Request(`http://localhost/api/fiches-dialogue/public?token=${encodeURIComponent(token)}`));
    check("Fiche ouverte par le parent", ctxRes.status === 200, `status=${ctxRes.status}`);
    const submit = await post({
      action: "submit",
      token,
      kind: "saisie",
      auteurLabel: "Parent test",
      signature: { name: "Parent test", method: "pad" },
      payload: { values: { destination: "2nde_pro" }, comment: "Projet bac pro" },
    });
    const ficheApres = await wf.getFdFiche(etablissementId, match.ficheId);
    check(
      "Vœu « 2nde professionnelle » enregistré, 2e parent sollicité",
      submit.status === 200 && ficheApres?.statut === "en_attente_accord_parent2",
      `status=${submit.status}, statut=${ficheApres?.statut}`,
    );

    // 7. Le 2e parent confirme depuis son lien personnel.
    const [t2] = await db
      .select()
      .from(schema.fdToken)
      .where(and(eq(schema.fdToken.ficheId, match.ficheId), eq(schema.fdToken.purpose, "accord_parent2")))
      .orderBy(desc(schema.fdToken.createdAt))
      .limit(1);
    check("Lien de confirmation créé pour l’autre parent", !!t2 && t2.email !== tok.email, `adresse=${t2?.email}`);
    const accord = await post({ action: "parent_accord", token: t2.token, decision: "confirme" });
    const ficheFin = await wf.getFdFiche(etablissementId, match.ficheId);
    check(
      "Accord du 2e parent : fiche prête pour le conseil",
      accord.status === 200 && ficheFin?.statut === "saisie_recue",
      `status=${accord.status}, statut=${ficheFin?.statut}`,
    );
  } finally {
    console.log(`\nTOTAL : ${ok} OK / ${ko} KO`);
    await endTestDatabase(closeDb);
  }
  if (ko > 0) process.exitCode = 1;
}

main().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
