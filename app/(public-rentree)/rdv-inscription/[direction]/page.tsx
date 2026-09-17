import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getRdvInscriptionDirectionBySlug } from "@/app/lib/rdv-inscription-db";
import { listPublicSlotsForDirection } from "@/app/lib/rdv-inscription-service";
import RdvInscriptionPublicClient from "./RdvInscriptionPublicClient";

type PageProps = { params: Promise<{ direction: string }> };

function unavailablePage(slug: string, title = "Rendez-vous d’inscription") {
  return (
    <RdvInscriptionPublicClient
      directionSlug={slug}
      title={title}
      intro=""
      consentLabel=""
      location=""
      directionLabel={slug}
      directriceDisplayName={null}
      initialSlots={[]}
      initialError="Service temporairement indisponible. Réessayez dans quelques minutes."
    />
  );
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { direction } = await params;
  const slug = decodeURIComponent(direction || "").trim().toLowerCase();
  try {
    const dir = await getRdvInscriptionDirectionBySlug(slug, { activeOnly: true });
    const title = dir?.title || "Rendez-vous d’inscription";
    const label = dir?.label || slug;
    return {
      title: `${title} — ${label}`,
      description: "Prise de rendez-vous avec la direction pour une inscription.",
    };
  } catch {
    return {
      title: "Rendez-vous d’inscription",
      description: "Prise de rendez-vous avec la direction pour une inscription.",
    };
  }
}

export default async function RdvInscriptionPublicPage({ params }: PageProps) {
  const { direction } = await params;
  const slug = decodeURIComponent(direction || "").trim().toLowerCase();
  if (!slug) notFound();

  let dir;
  try {
    // Une direction active = page publique (plus de 404 si le switch global était oublié).
    dir = await getRdvInscriptionDirectionBySlug(slug, { activeOnly: true });
  } catch {
    return unavailablePage(slug);
  }
  if (!dir) notFound();

  // Aligne le coupe-circuit global si une direction active est visitée.
  try {
    const { getRdvInscriptionConfig, updateRdvInscriptionConfig } = await import(
      "@/app/lib/rdv-inscription-db"
    );
    const cfg = await getRdvInscriptionConfig();
    if (!cfg.enabled) {
      await updateRdvInscriptionConfig({ enabled: true });
    }
  } catch {
    /* non bloquant */
  }

  const listed = await listPublicSlotsForDirection(slug);

  return (
    <RdvInscriptionPublicClient
      directionSlug={slug}
      title={listed.ok ? listed.configTitle : dir.title}
      intro={listed.ok ? listed.intro : dir.intro}
      consentLabel={listed.ok ? listed.consentLabel : dir.consentLabel}
      location={listed.ok ? listed.location : dir.location}
      directionLabel={listed.ok ? listed.directionLabel : dir.label}
      directriceDisplayName={
        listed.ok ? listed.directriceDisplayName : dir.directriceDisplayName
      }
      initialSlots={
        listed.ok
          ? listed.slots.map((s) => ({
              eventId: s.eventId,
              startAt: s.startAt,
              endAt: s.endAt,
              title: s.title,
            }))
          : []
      }
      initialError={listed.ok ? null : listed.error}
    />
  );
}
