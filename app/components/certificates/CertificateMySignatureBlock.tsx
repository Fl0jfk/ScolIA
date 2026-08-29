"use client";

import UserSignaturePad from "@/app/components/account/UserSignaturePad";

export default function CertificateMySignatureBlock() {
  return (
    <UserSignaturePad
      apiPath="/api/account/my-signature"
      title="Mon paraphe (certificats)"
      description="Votre signature est enregistrée dans Mon compte → Sécurité → Ma signature et réutilisée ici automatiquement."
      successMessage="Paraphe enregistré — réutilisable sur tous vos certificats et conventions."
    />
  );
}
