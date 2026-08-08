import { redirect } from "next/navigation";

/** Ancienne URL toolbox — redirige vers le hub Événements. */
export default function LegacySecretSantaRedirect() {
  redirect("/etablissement/evenements/secret-santa");
}
