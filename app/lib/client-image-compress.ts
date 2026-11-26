/**
 * Compression de photos côté NAVIGATEUR avant envoi (même principe que le panneau « communication parents ») :
 * redimensionnement à 1600 px, JPEG. Le canvas ne recopie pas les métadonnées ; le serveur nettoie de toute façon
 * (EXIF/GPS) et ne se fie jamais à ce que le navigateur annonce.
 */

export type CompressedPhoto = { id: string; blob: Blob; name: string; previewUrl: string };

const MAX_SIDE = 1600;
const TARGET_BYTES = 900_000;

function toJpeg(canvas: HTMLCanvasElement, quality: number): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("Compression échouée"))), "image/jpeg", quality);
  });
}

export async function compressPhotoFile(file: File): Promise<CompressedPhoto> {
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file, { imageOrientation: "from-image" });
  } catch {
    throw new Error(`${file.name || "Photo"} : format non lisible par ce navigateur (essayez JPEG ou PNG).`);
  }
  const scale = Math.min(1, MAX_SIDE / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(bitmap.width * scale));
  canvas.height = Math.max(1, Math.round(bitmap.height * scale));
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    bitmap.close();
    throw new Error("Canvas indisponible");
  }
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  let blob = await toJpeg(canvas, 0.78);
  if (blob.size > TARGET_BYTES) blob = await toJpeg(canvas, 0.6);
  return {
    id: `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    blob,
    name: (file.name || "photo").replace(/\.\w+$/, "") + ".jpg",
    previewUrl: URL.createObjectURL(blob),
  };
}
