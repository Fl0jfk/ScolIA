import { redirect } from "next/navigation";
import { consumeRdvRescheduleToken } from "@/app/lib/rdv-inscription-service";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

type PageProps = {
  searchParams: Promise<{ token?: string }>;
};

/** Lien mail « choisir un autre créneau » : pose le cookie gate et redirige. */
export default async function RdvInscriptionRebookPage({ searchParams }: PageProps) {
  const sp = await searchParams;
  const token = String(sp.token || "").trim();

  if (!token) {
    redirect(await tenantAbsolutePath("/rdv-inscription/lycee?email_error=invalid"));
  }

  const result = await consumeRdvRescheduleToken(token);
  if (!result.ok) {
    const slug = result.directionSlug || "lycee";
    redirect(
      await tenantAbsolutePath(
        `/rdv-inscription/${encodeURIComponent(slug)}?email_error=${encodeURIComponent(result.error)}`,
      ),
    );
  }

  redirect(await tenantAbsolutePath(result.redirectPath));
}
