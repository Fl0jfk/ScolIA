import type { MessagingMessageType } from "@/app/lib/messaging/types";

export const MESSAGE_TYPES = [
  "text",
  "image",
  "file",
  "audio",
  "video",
  "system",
] as const satisfies readonly MessagingMessageType[];

export const MAX_FILE_BYTES = 25 * 1024 * 1024;
export const MAX_VIDEO_BYTES = 50 * 1024 * 1024;
export const MAX_AUDIO_BYTES = 25 * 1024 * 1024;
export const RETENTION_DAYS = 365;

export const ALLOWED_MIME_TYPES = new Set<string>([
  "image/jpeg",
  "image/jpg",
  "image/png",
  "image/gif",
  "image/webp",
  "image/heic",
  "image/heif",
  "video/mp4",
  "video/webm",
  "video/quicktime",
  "audio/mpeg",
  "audio/mp3",
  "audio/mp4",
  "audio/webm",
  "audio/ogg",
  "audio/wav",
  "audio/x-wav",
  "audio/aac",
  "application/pdf",
  "text/plain",
  "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/vnd.ms-excel",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "application/vnd.ms-powerpoint",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  "application/vnd.oasis.opendocument.text",
  "application/vnd.oasis.opendocument.spreadsheet",
  "application/zip",
]);

export const ACCEPT_FILE_INPUT =
  "image/*,.pdf,.doc,.docx,.xls,.xlsx,.odt,.ods,.txt,audio/*,video/mp4,video/webm,video/quicktime";

export const REACTION_EMOJIS = ["👍", "❤️", "😂", "😮", "😢", "🙏"] as const;

export function isVideoMime(mime: string): boolean {
  return mime.toLowerCase().startsWith("video/");
}

export function isAllowedMime(mime: string): boolean {
  if (!mime) return false;
  if (ALLOWED_MIME_TYPES.has(mime)) return true;
  if (mime.startsWith("image/")) return true;
  if (mime.startsWith("video/")) return true;
  if (mime.startsWith("audio/")) return true;
  return false;
}

export function maxBytesForMime(mime: string): number {
  const m = mime.toLowerCase();
  if (m.startsWith("video/")) return MAX_VIDEO_BYTES;
  if (m.startsWith("audio/")) return MAX_AUDIO_BYTES;
  return MAX_FILE_BYTES;
}

export function messageTypeFromMime(mime: string): "image" | "file" | "audio" | "video" {
  const m = mime.toLowerCase();
  if (m.startsWith("image/")) return "image";
  if (m.startsWith("audio/")) return "audio";
  if (m.startsWith("video/")) return "video";
  return "file";
}

export function isAllowedMessageType(value: string): value is MessagingMessageType {
  return (MESSAGE_TYPES as readonly string[]).includes(value);
}

const URL_RE = /https?:\/\/[^\s<>"')]+/gi;

export function extractUrls(text: string): string[] {
  const matches = text.match(URL_RE);
  if (!matches) return [];
  return [...new Set(matches.map((u) => u.replace(/[.,;:!?)]+$/, "")))];
}
