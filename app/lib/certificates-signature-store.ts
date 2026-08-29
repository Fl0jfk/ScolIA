import {
  loadUserSignatureBytes,
  parseSignaturePngBase64,
  saveUserSignature,
} from "@/app/lib/user-signature-store";

export async function loadCertificateProfSignatureBytes(
  externalUserId: string,
): Promise<Uint8Array | null> {
  return loadUserSignatureBytes(externalUserId);
}

export async function saveCertificateProfSignature(
  externalUserId: string,
  pngBytes: Buffer,
): Promise<void> {
  await saveUserSignature(externalUserId, pngBytes);
}

export const parseCertificatePngBase64 = parseSignaturePngBase64;
