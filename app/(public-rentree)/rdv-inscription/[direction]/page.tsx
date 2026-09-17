import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getRdvInscriptionConfig, getRdvInscriptionDirectionBySlug } from "@/app/lib/rdv-inscription-db";
import { listPublicSlotsForDirection } from "@/app/lib/rdv-inscription-service";
import RdvInscriptionPublicClient from "./RdvInscriptionPublicClient";

type PageProps = { params: Promise<{ direction: string }> };

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { direction } = await params;
  const slug = decodeURIComponent(direction || "").trim().toLowerCase();
  try {
    const [config, dir] = await Promise.all([
      getRdvInscriptionConfig(),
      getRdvInscriptionDirectionBySlug(slug, { activeOnly: true }),
    ]);
    const title = config?.title || "Rendez-vous d’inscription";
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

  let config;
  let dir;
  try {
    config = await getRdvInscriptionConfig();
    if (!config.enabled) notFound();
    dir = await getRdvInscriptionDirectionBySlug(slug, { activeOnly: true });
    if (!dir) notFound();
  } catch {
    return (
      <RdvInscriptionPublicClient
        directionSlug={slug}
        title="Rendez-vous d’inscription"
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

  const listed = await listPublicSlotsForDirection(slug);

  if (!listed.ok && listed.status === 404) notFound();

  return (
    <RdvInscriptionPublicClient
      directionSlug={slug}
      title={listed.ok ? listed.configTitle : config.title}
      intro={listed.ok ? listed.intro : config.intro}
      consentLabel={listed.ok ? listed.consentLabel : config.consentLabel}
      location={listed.ok ? listed.location : config.location}
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
