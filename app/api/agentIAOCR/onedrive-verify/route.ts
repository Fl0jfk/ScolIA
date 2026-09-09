import { POST as verifyOneDriveToken } from "@/app/api/onedrive/verify/route";

/** @deprecated Preferer `/api/onedrive/verify` (hors garde module OCR). */
export const POST = verifyOneDriveToken;
