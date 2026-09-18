import { redirect } from "next/navigation";

/** Ancienne URL — redirection vers `/photocopies` (stockage S3 inchangé). */
export default function PhotocopiesCouleurRedirectPage() {
  redirect("/photocopies");
}
