import { NextResponse } from "next/server";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { openPhotocopieReadyToken } from "@/app/lib/photocopies-couleur-ready-token";
import type { PhotoCopieRecord } from "@/app/lib/photocopies-couleur-types";

const INDEX_KEY = "photocopies-couleur/index.json";

async function getIndex(): Promise<PhotoCopieRecord[]> {
  const hit = await getJson<PhotoCopieRecord[]>(INDEX_KEY);
  return hit?.data ?? [];
}

async function saveIndex(rows: PhotoCopieRecord[]) {
  await putJson(INDEX_KEY, rows);
}

function renderHtmlPage(opts: {
  title: string;
  message: string;
  ok: boolean;
  detail?: string;
}) {
  const color = opts.ok ? "#059669" : "#e11d48";
  return `<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>${opts.title}</title>
  <style>
    body { font-family: system-ui, sans-serif; background: #f8fafc; margin: 0; padding: 2rem; color: #0f172a; }
    .card { max-width: 32rem; margin: 4rem auto; background: white; border-radius: 1rem; padding: 2rem; box-shadow: 0 20px 50px -32px rgba(15,23,42,0.35); }
    h1 { font-size: 1.35rem; margin: 0 0 1rem; color: ${color}; }
    p { line-height: 1.55; margin: 0 0 0.75rem; }
    .detail { font-size: 0.9rem; color: #64748b; }
  </style>
</head>
<body>
  <div class="card">
    <h1>${opts.title}</h1>
    <p>${opts.message}</p>
    ${opts.detail ? `<p class="detail">${opts.detail}</p>` : ""}
  </div>
</body>
</html>`;
}

/** Lien e-mail gestionnaire impressions — marque la demande comme prête et notifie l'enseignant. */
export async function GET(req: Request) {
  const url = new URL(req.url);
  const token = url.searchParams.get("t")?.trim() || "";

  const payload = openPhotocopieReadyToken(token);
  if (!payload) {
    return new NextResponse(
      renderHtmlPage({
        ok: false,
        title: "Lien invalide ou expiré",
        message: "Ce lien n'est plus valide. Contactez l'équipe informatique si le problème persiste.",
      }),
      { status: 400, headers: { "Content-Type": "text/html; charset=utf-8" } },
    );
  }

  try {
    const all = await getIndex();
    const idx = all.findIndex((r) => r.id === payload.id);
    if (idx < 0) {
      return new NextResponse(
        renderHtmlPage({
          ok: false,
          title: "Demande introuvable",
          message: "Cette demande de photocopies n'existe plus ou a été supprimée.",
        }),
        { status: 404, headers: { "Content-Type": "text/html; charset=utf-8" } },
      );
    }

    const current = all[idx];
    if (current.status === "PRETE") {
      return new NextResponse(
        renderHtmlPage({
          ok: true,
          title: "Déjà marquée comme prête",
          message: `Les photocopies de ${current.createdBy.name} avaient déjà été signalées comme prêtes.`,
          detail: `${current.nombrePhotocopies} exemplaire(s) — ${current.etablissement}`,
        }),
        { status: 200, headers: { "Content-Type": "text/html; charset=utf-8" } },
      );
    }

    if (current.status !== "ACCEPTEE") {
      return new NextResponse(
        renderHtmlPage({
          ok: false,
          title: "Demande non traitable",
          message:
            current.status === "REFUSEE"
              ? "Cette demande a été refusée par la direction et ne peut pas être marquée prête."
              : "Cette demande n'a pas encore été acceptée par la direction.",
        }),
        { status: 400, headers: { "Content-Type": "text/html; charset=utf-8" } },
      );
    }

    const updated: PhotoCopieRecord = {
      ...current,
      status: "PRETE",
      updatedAt: new Date().toISOString(),
      readyAt: new Date().toISOString(),
      readyBy: "Gestionnaire impressions",
    };
    all[idx] = updated;
    await saveIndex(all);

    const creatorEmail = updated.createdBy.email?.trim();
    const intranetLink = await tenantAbsolutePath("/photocopies-couleur");
    const smtp = await getTenantSmtpConfig();
    const transporter = smtp ? await createTenantTransporter() : null;
    if (transporter && smtp && creatorEmail) {
      try {
        await transporter.sendMail({
          from: `"Demandes photocopies" <${smtp.user}>`,
          to: creatorEmail,
          subject: "Vos photocopies couleur sont prêtes",
          text: [
            `Bonjour ${updated.createdBy.name},`,
            ``,
            `Vos photocopies couleur sont prêtes à être retirées.`,
            ``,
            `Établissement : ${updated.etablissement}`,
            `Nombre : ${updated.nombrePhotocopies}`,
            `Classes / matière : ${updated.classesOuMatiere}`,
            ``,
            `Consulter vos demandes : ${intranetLink}`,
            ``,
            `Cordialement,`,
            `La Providence Nicolas Barré`,
          ].join("\n"),
          html: `<p>Bonjour ${updated.createdBy.name},</p>
<p><strong>Vos photocopies couleur sont prêtes</strong> à être retirées.</p>
<ul>
<li>Établissement : ${updated.etablissement}</li>
<li>Nombre : ${updated.nombrePhotocopies}</li>
<li>Classes / matière : ${updated.classesOuMatiere}</li>
</ul>
<p><a href="${intranetLink}">Voir mes demandes sur l'intranet</a></p>`,
        });
      } catch (mailErr) {
        console.error("[photocopies-couleur/mark-ready] mail demandeur:", mailErr);
      }
    }

    return new NextResponse(
      renderHtmlPage({
        ok: true,
        title: "Photocopies marquées prêtes",
        message: `${updated.createdBy.name} a été notifié(e) par e-mail.`,
        detail: `${updated.nombrePhotocopies} exemplaire(s) — ${updated.etablissement}`,
      }),
      { status: 200, headers: { "Content-Type": "text/html; charset=utf-8" } },
    );
  } catch (e) {
    console.error("[photocopies-couleur/mark-ready] GET", e);
    return new NextResponse(
      renderHtmlPage({
        ok: false,
        title: "Erreur technique",
        message: "Impossible de mettre à jour la demande pour le moment. Réessayez dans quelques instants.",
      }),
      { status: 500, headers: { "Content-Type": "text/html; charset=utf-8" } },
    );
  }
}
