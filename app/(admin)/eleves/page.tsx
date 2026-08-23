import { redirect } from "next/navigation";

/** Legacy hub Élèves → Administratif / Services / Vie scolaire selon tab. */
export default async function ElevesLegacyRedirect({
  searchParams,
}: {
  searchParams: Promise<{ tab?: string }>;
}) {
  const { tab } = await searchParams;
  if (tab === "internat") redirect("/vie-scolaire?tab=internat");
  if (tab === "travels" || tab === "stages") redirect(`/services?tab=${tab}`);
  if (tab) redirect(`/administratif?tab=${tab}`);
  redirect("/administratif");
}
