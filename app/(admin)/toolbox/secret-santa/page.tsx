import { redirect } from "next/navigation";

export default function LegacySecretSantaRedirect() {
  redirect("/secret-santa");
}
