import { redirect } from "next/navigation";

/** Legacy hub Élèves → Administratif / Services / Vie scolaire selon tab. */
export default async function ElevesLegacyRedirect({
  searchParams,
}: {
  searchParams: Promise<{ tab?: string }>;
}) {
  const { tab } = await searchParams;
  if (tab === "internat") redirect("/vie-scolaire?tab=internat");
  if (tab === "travels") redirect("/services?tab=travels");
  if (tab === "stages") redirect("/administratif?tab=stages");
  if (tab) redirect(`/administratif?tab=${tab}`);
  redirect("/administratif");
}
