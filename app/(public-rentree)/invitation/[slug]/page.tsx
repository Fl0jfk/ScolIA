import type { Metadata } from "next";
import { notFound } from "next/navigation";
import InvitationPublicClient from "@/app/components/invitation/InvitationPublicClient";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  getInvitationPageBySlug,
  sumOuiPresentCount,
} from "@/app/lib/invitation-db";
import type { InvitationPagePublic } from "@/app/lib/invitation-types";

type Props = { params: Promise<{ slug: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  try {
    const page = await getInvitationPageBySlug(slug);
    if (!page || !page.enabled) return { title: "Invitation" };
    return {
      title: page.title,
      description: page.intro?.slice(0, 160) || "Invitation de l’établissement",
    };
  } catch {
    return { title: "Invitation" };
  }
}

export default async function InvitationPublicPage({ params }: Props) {
  const { slug } = await params;
  const page = await getInvitationPageBySlug(slug);
  if (!page || !page.enabled) notFound();

  const [used, bundle] = await Promise.all([
    sumOuiPresentCount(page.id),
    loadAppConfig(),
  ]);
  const placesRemaining = Math.max(0, page.maxTotalPersons - used);
  const schoolName = bundle.identity.shortName || bundle.identity.name || "";

  const publicPage: InvitationPagePublic = {
    slug: page.slug,
    title: page.title,
    intro: page.intro,
    theme: page.theme,
    startsAt: page.startsAt,
    endsAt: page.endsAt,
    location: page.location,
    diplomaMode: page.diplomaMode,
    maxPersonsPerEleve: page.maxPersonsPerEleve,
    placesRemaining,
    schoolName,
  };

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        rel="stylesheet"
        href="https://fonts.googleapis.com/css2?family=Cormorant+Garamond:wght@500;600;700&family=Figtree:wght@400;500;600;700&display=swap"
      />
      <InvitationPublicClient page={publicPage} />
    </>
  );
}
