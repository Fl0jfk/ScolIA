import { redirect } from "next/navigation";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

type PageProps = {
  searchParams: Promise<{ token?: string; direction?: string }>;
};

/**
 * Anciens mails pointaient ici. On renvoie vers la Route Handler
 * (seule surface autorisée à poser le cookie gate sous Next 16).
 */
export default async function RdvInscriptionRebookPage({ searchParams }: PageProps) {
  const sp = await searchParams;
  const params = new URLSearchParams();
  const token = String(sp.token || "").trim();
  const direction = String(sp.direction || "").trim().toLowerCase();
  if (token) params.set("token", token);
  if (direction) params.set("direction", direction);
  redirect(
    await tenantAbsolutePath(
      `/api/rdv-inscription/rebook${params.toString() ? `?${params.toString()}` : ""}`,
    ),
  );
}
