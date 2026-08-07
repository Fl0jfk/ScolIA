import { redirect } from "next/navigation";

/** Ancienne URL — utilisateurs intégrés dans Paramètres généraux. */
export default function MembresPage() {
  redirect("/parametres?tab=utilisateurs");
}
