import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from 'next/server';

import { isValidTravelsReopenFromValideStatus } from "@/app/lib/travels-direction-permissions";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import IMAGE_CATALOG from "./image-catalog.json";
import { notifyComptaTravelsPhase, type TravelsTripForNotify } from "@/app/lib/travels-notify";
import { applyTravelsOwnerAssignment } from "@/app/lib/travels-owner-server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { canSignTravelsDirectionForEtab, resolveDirectorForEstablishment } from "@/app/lib/establishments";

/** Le client fait foi (suppressions incluses) ; on n'ajoute depuis S3 que si le client n'envoie pas la liste. */
function mergeReceivedDevis(fromClient: unknown, fromS3: unknown): unknown[] {
  if (Array.isArray(fromClient)) return fromClient;
  return Array.isArray(fromS3) ? fromS3 : [];
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
    try {
    const body = await req.json();
    const suppressNewTripEmail = Boolean(body.suppressNewTripEmail);
    const tripId = body.id;
    const innerData = body.data?.data || {}; 
    const title = innerData.title || "Titre introuvable";
    const destination = innerData.destination || "Destination introuvable";
    const objectToSave = body.data; 
    if (!objectToSave.imageUrl) {
      try {
        const mistralKey = await getMistralApiKey();
        if (mistralKey) {
        const catalogSummary = IMAGE_CATALOG.map(i => `${i.id} (${i.label})`).join(", ");
        const response = await fetch("https://api.mistral.ai/v1/chat/completions", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "Authorization": `Bearer ${mistralKey}`
          },
          body: JSON.stringify({
            model: "mistral-small-latest",
            messages: [
              { 
                role: "system", 
                content: `Choisis l'ID exact parmis : ${catalogSummary}. Réponds uniquement par l'ID. Sinon "img_default".` 
              },
              { 
                role: "user", 
                content: `DONNÉES À ANALYSER : - TITRE : "${title}" - LIEU : "${destination}"` 
              }
            ],
            temperature: 0
          })
        });
        const resData = await response.json();
        const mistralChoice = resData.choices?.[0]?.message?.content?.trim();
        const matchedImage = IMAGE_CATALOG.find(img => 
          img.id.toLowerCase().replace(/[^a-z0-9]/g, '') === mistralChoice?.toLowerCase().replace(/[^a-z0-9]/g, '')
        ) || IMAGE_CATALOG.find(img => img.id === "img_default") || IMAGE_CATALOG[0];
        objectToSave.imageUrl = matchedImage.url;
        objectToSave.imageConfigId = matchedImage.id;
        }
      } catch (err) { console.error("Erreur IA:", err)}
    }
    const tripRel = `travels/${tripId}.json`;
    const existingHit = await getJson<Record<string, unknown>>(tripRel);
    const existingOnS3 = existingHit?.data ?? null;
    const me = await safeCurrentUser();
    const ownerGate = await applyTravelsOwnerAssignment(
      objectToSave as Record<string, unknown>,
      me
        ? {
            id: me.id,
            fullName: me.fullName,
            primaryEmailAddress: me.primaryEmailAddress,
            publicMetadata: me.publicMetadata as Record<string, unknown>,
          }
        : null,
      existingOnS3,
    );
    if (!ownerGate.ok) {
      return NextResponse.json({ error: ownerGate.error }, { status: ownerGate.status });
    }
    if (existingOnS3 && Array.isArray(existingOnS3.receivedDevis)) {
      objectToSave.receivedDevis = mergeReceivedDevis(
        objectToSave.receivedDevis,
        existingOnS3.receivedDevis
      );
    }
    const existingInner = (existingOnS3?.data || {}) as Record<string, unknown>;
    const saveInner = (objectToSave.data || {}) as Record<string, unknown>;
    if (existingInner.cuisineOrderSentAt && !saveInner.cuisineOrderSentAt) {
      saveInner.cuisineOrderSentAt = existingInner.cuisineOrderSentAt;
      if (existingInner.cuisineOrderSnapshot && !saveInner.cuisineOrderSnapshot) {
        saveInner.cuisineOrderSnapshot = existingInner.cuisineOrderSnapshot;
      }
      if (existingInner.cuisineAmendments && !saveInner.cuisineAmendments) {
        saveInner.cuisineAmendments = existingInner.cuisineAmendments;
      }
      objectToSave.data = saveInner;
    }
    if (Array.isArray(saveInner.participantEleves)) {
      const { applyParticipantElevesToTripData } = await import("@/app/lib/travels-eleves-list");
      const participants = saveInner.participantEleves as import("@/app/lib/travels-types").TravelsParticipantEleve[];
      const synced = applyParticipantElevesToTripData(
        saveInner as import("@/app/lib/travels-types").TravelsTripData,
        participants,
      );
      saveInner.participantEleves = synced.participantEleves;
      saveInner.nbEleves = synced.nbEleves;
      saveInner.classes = synced.classes;
      if (!saveInner.listeElevesStatus) saveInner.listeElevesStatus = "draft";
      objectToSave.data = saveInner;
    }
    const previousStatus = existingOnS3 && typeof existingOnS3.status === "string" ? existingOnS3.status : null;
    const newStatus = typeof objectToSave.status === "string" ? objectToSave.status : "";
    if (previousStatus === "VALIDE" && newStatus !== "VALIDE" && newStatus !== previousStatus) {
      if (!isValidTravelsReopenFromValideStatus(newStatus)) {
        return NextResponse.json({ error: "Étape de réouverture invalide." }, { status: 400 });
      }
      const me = await safeCurrentUser();
      const innerForPerm = (objectToSave.data as { etablissement?: string } | undefined) || {};
      const etab =
        typeof innerForPerm.etablissement === "string"
          ? innerForPerm.etablissement
          : typeof (existingOnS3?.data as { etablissement?: string } | undefined)?.etablissement === "string"
            ? (existingOnS3!.data as { etablissement: string }).etablissement
            : null;
      if (!(await canSignTravelsDirectionForEtab(me,  etab))) {
        return NextResponse.json(
          { error: "Seule la direction de l'établissement concerné peut réouvrir un dossier finalisé." },
          { status: 403 },
        );
      }
    }
    objectToSave.updatedAt = new Date().toISOString();
    await putJson(tripRel, objectToSave);
    const indexHit = await getJson<unknown[]>("travels/index.json");
    let currentIndex: unknown[] = Array.isArray(indexHit?.data) ? indexHit.data : [];
    const tripSummary = {
      id: tripId,
      ownerName: objectToSave.ownerName,
      ownerId: objectToSave.ownerId,
      status: objectToSave.status,
      type: objectToSave.type,
      createdAt: objectToSave.createdAt || new Date().toISOString(),
      data: {
        title: title,
        destination: destination,
        imageUrl: objectToSave.imageUrl,
        nbEleves: innerData.nbEleves,
        nbAccompagnateurs: innerData.nbAccompagnateurs,
        nomsAccompagnateurs: innerData.nomsAccompagnateurs || [],
        classes: innerData.classes || [],
        piqueNique: innerData.piqueNique || false,
        piqueNiqueDetails: innerData.piqueNiqueDetails || null,
        date: innerData.date || null, 
        startDate: innerData.startDate || innerData.date || null,
        endDate: innerData.endDate || innerData.date || null,
        startTime: innerData.startTime || null,
        endTime: innerData.endTime || null,
        needsBus: innerData.needsBus || false,
        transportRequest: innerData.transportRequest || null,
        objectifs: innerData.objectifs || innerData.pedagogicalObjectives || null,
        coutTotal: innerData.coutTotal,
        etablissement: innerData.etablissement || null,
        recurrenceSeriesId: innerData.recurrenceSeriesId || null,
        recurrenceIndex: innerData.recurrenceIndex ?? null,
        recurrenceTotal: innerData.recurrenceTotal ?? null,
      }
    };
    const existingIndex = currentIndex.findIndex((t: any) => t.id === tripId);
    const isNewProject = existingIndex === -1;
    if (existingIndex > -1) { currentIndex[existingIndex] = tripSummary;
    } else { currentIndex.push(tripSummary);
    }
    await putJson("travels/index.json", currentIndex);
    try {
      const { syncTripActualite } = await import("@/app/lib/brain-ai/sync/knowledge-writer");
      const { TRAVELS_STATUS_LABELS } = await import("@/app/lib/travels-types");
      const status = String(objectToSave.status || "");
      const start = innerData.startDate || innerData.date || "";
      const end = innerData.endDate || innerData.date || start;
      void syncTripActualite({
        id: String(tripId),
        title: String(title || ""),
        dates: start && end && start !== end ? `${start} → ${end}` : String(start || ""),
        classes: Array.isArray(innerData.classes)
          ? innerData.classes.join(", ")
          : String(innerData.classes || ""),
        statusLabel: TRAVELS_STATUS_LABELS[status] || status,
      });
    } catch (syncErr) {
      console.warn("[travels/update] sync actualite failed", syncErr);
    }
    if (newStatus === "EN_ATTENTE_COMPTA" && previousStatus !== "EN_ATTENTE_COMPTA") {
      try {
        await notifyComptaTravelsPhase({ tripId,
          trip: objectToSave as TravelsTripForNotify,
          previousStatus,
        });
      } catch (mailErr) {
        console.error("Erreur notification compta Travels:", mailErr);
      }
    }
    if (isNewProject && !suppressNewTripEmail) {
      try {
        const director = await resolveDirectorForEstablishment( innerData.etablissement);
        const smtp = await getTenantSmtpConfig();
        const transporter = smtp ? await createTenantTransporter() : null;
        if (transporter && smtp) {
        const creatorName = objectToSave.ownerName || "Enseignant";
        const dateInfo = innerData.startDate || innerData.endDate ? `du ${innerData.startDate || innerData.date || "—"} au ${innerData.endDate || innerData.date || "—"}` : (innerData.date || "—");
        await transporter.sendMail({
          from: `"Plateforme Voyages" <${smtp.user}>`,
          to: director.email,
          subject: `Nouvelle demande de sortie — ${innerData.etablissement || "Groupe Scolaire"} — ${title}`,
          text: [
            `Bonjour ${director.directrice},`,
            ``,
            `Un nouveau projet de sortie a été créé sur DocsLapro et nécessite votre suivi.`,
            ``,
            `Établissement ciblé : ${innerData.etablissement || "Groupe Scolaire"}`,
            `Créé par : ${creatorName}`,
            `Titre : ${title}`,
            `Destination : ${destination}`,
            `Dates : ${dateInfo}`,
            `Effectif : ${innerData.nbEleves || "—"} élèves / ${innerData.nbAccompagnateurs || "—"} accompagnateurs`,
            ``,
            `Vous pouvez consulter le dossier dans l'espace sortie scolaire de DocsLapro.`,
            ``,
            `Cordialement,`,
            `Plateforme Voyages`,
          ].join("\n"),
        });
        }
      } catch (mailErr) { console.error("Erreur notification direction:", mailErr)}
    }
    return NextResponse.json({
      success: true,
      imageUrl: objectToSave.imageUrl,
      imageConfigId: objectToSave.imageConfigId,
    });
  } catch (error) {
    console.error("ERREUR S3:", error);
    return NextResponse.json({ error: "Échec" }, { status: 500 });
  }
}