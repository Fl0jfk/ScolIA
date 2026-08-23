import { redirect } from "next/navigation";

/** Legacy hub Établissement → Administratif. */
export default async function EtablissementLegacyRedirect({
  searchParams,
}: {
  searchParams: Promise<{ tab?: string }>;
}) {
  const { tab } = await searchParams;
  if (tab) redirect(`/administratif?tab=${tab}`);
  redirect("/administratif");
}
