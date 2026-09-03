/**
 * Colonne booked_by_user_id pour conserver le compte booker (réservation pour autrui).
 */
ALTER TABLE "reservation_room_booking"
  ADD COLUMN IF NOT EXISTS "booked_by_user_id" text;
