import {
  loadUserSignatureBytes,
  parseSignaturePngBase64,
  saveUserSignature,
} from "@/app/lib/user-signature-store";

export async function loadReferentSignatureBytes(externalUserId: string): Promise<Uint8Array | null> {
  return loadUserSignatureBytes(externalUserId);
}

export async function saveReferentSignature(externalUserId: string, pngBytes: Buffer): Promise<void> {
  await saveUserSignature(externalUserId, pngBytes);
}

export const parsePngBase64 = parseSignaturePngBase64;
