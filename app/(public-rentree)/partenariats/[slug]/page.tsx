import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { partenariatsBody as body, partenariatsDisplay as display } from "@/app/fonts/partenariats";
import PartenariatPublicDetailClient from "@/app/components/partenariats/PartenariatPublicDetailClient";
import { getPublicPartenariatDetail } from "@/app/lib/partenariats-db";

type Props = { params: Promise<{ slug: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  try {
    const offre = await getPublicPartenariatDetail(slug);
    if (!offre) return { title: "Partenariat" };
    return {
      title: offre.title,
      description: offre.shortDescription?.slice(0, 160) || "Partenariat de l’établissement",
    };
  } catch {
    return { title: "Partenariat" };
  }
}

export default async function PartenariatPublicDetailPage({ params }: Props) {
  const { slug } = await params;
  const offre = await getPublicPartenariatDetail(slug);
  if (!offre) notFound();

  return (
    <div className={`${display.variable} ${body.variable} font-[family-name:var(--font-partenariats-body)]`}>
      <PartenariatPublicDetailClient offre={offre} />
    </div>
  );
}
