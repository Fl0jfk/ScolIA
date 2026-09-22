import "server-only";

import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleveDocument, eleveScolarite } from "@/db/schema";
import { defaultAccompagnementDocumentTitle } from "@/app/lib/eleve-pap";
import type { RdvInscriptionBookingRow } from "@/app/lib/rdv-inscription-types";

/** Après confirmation RDV : PAP → tiroir santé + établissement d’origine sur scolarité. */
export async function attachRdvBookingExtrasToEleve(opts: {
  etablissementId: string;
  eleveId: string;
  booking: RdvInscriptionBookingRow;
}): Promise<void> {
  const db = getDb();
  const { booking, eleveId, etablissementId } = opts;

  if (booking.etablissementOrigineLabel) {
    const rows = await db
      .select({ id: eleveScolarite.id })
      .from(eleveScolarite)
      .where(
        and(
          eq(eleveScolarite.etablissementId, etablissementId),
          eq(eleveScolarite.eleveId, eleveId),
        ),
      )
      .orderBy(desc(eleveScolarite.createdAt))
      .limit(1);
    if (rows[0]) {
      await db
        .update(eleveScolarite)
        .set({
          etablissementPrecedent: booking.etablissementOrigineLabel,
          updatedAt: new Date(),
        })
        .where(eq(eleveScolarite.id, rows[0].id));
    } else {
      await db.insert(eleveScolarite).values({
        etablissementId,
        eleveId,
        classe: booking.niveauLabel,
        statut: "prevue",
        etablissementPrecedent: booking.etablissementOrigineLabel,
      });
    }
  }

  if (booking.papS3Key) {
    const title = defaultAccompagnementDocumentTitle("pap");
    const mimeType = booking.papMimeType || "application/pdf";
    try {
      const { looksLikePdfUpload, stripBlankPagesInS3Object } = await import(
        "@/app/lib/pdf-strip-blank-pages"
      );
      if (
        looksLikePdfUpload({
          mimeType,
          s3Key: booking.papS3Key,
        })
      ) {
        await stripBlankPagesInS3Object(booking.papS3Key);
      }
    } catch (stripErr) {
      console.warn("[rdv-inscription] strip blank pages PAP (non bloquant)", stripErr);
    }
    await db.insert(eleveDocument).values({
      etablissementId,
      eleveId,
      tiroir: "sante",
      title,
      mimeType,
      s3Key: booking.papS3Key,
      confidentialite: "sante",
      source: "rdv-inscription",
    });
  }
}
