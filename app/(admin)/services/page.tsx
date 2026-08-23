import { redirect } from "next/navigation";

/** Legacy hub Services → Administratif (ou Vie scolaire pour salles). */
export default async function ServicesLegacyRedirect({
  searchParams,
}: {
  searchParams: Promise<{ tab?: string }>;
}) {
  const { tab } = await searchParams;
  if (tab === "salles" || tab === "transversal") redirect(`/vie-scolaire?tab=${tab}`);
  if (tab) redirect(`/administratif?tab=${tab}`);
  redirect("/administratif");
}
