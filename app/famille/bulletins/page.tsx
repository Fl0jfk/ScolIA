import FamilleBulletinsClient from "@/app/components/famille/FamilleBulletinsClient";

export const metadata = {
  title: "Bulletins — Espace famille",
};

/** Portail famille V1 (web de test — cible prod = app mobile). */
export default function FamilleBulletinsPage() {
  return <FamilleBulletinsClient />;
}
