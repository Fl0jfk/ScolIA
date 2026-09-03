/**
 * Salles & réservations — tables typées (plus d’EAV / document JSON).
 */
import {
  boolean,
  index,
  integer,
  pgTable,
  primaryKey,
  text,
  timestamp,
  uuid,
} from "drizzle-orm/pg-core";
import { etablissement } from "./etablissement-table";

export const reservationRoom = pgTable(
  "reservation_room",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    id: text("id").notNull(),
    name: text("name").notNull(),
    building: text("building"),
    sortOrder: integer("sort_order").notNull().default(0),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.id], name: "reservation_room_pk" }),
    index("reservation_room_etab_idx").on(t.etablissementId),
  ],
);

export const reservationRoomBooking = pgTable(
  "reservation_room_booking",
  {
    etablissementId: uuid("etablissement_id")
      .notNull()
      .references(() => etablissement.id, { onDelete: "cascade" }),
    id: text("id").notNull(),
    roomId: text("room_id").notNull(),
    groupId: text("group_id"),
    userId: text("user_id").notNull().default(""),
    firstName: text("first_name").notNull().default(""),
    lastName: text("last_name").notNull().default(""),
    bookedByFirstName: text("booked_by_first_name").notNull().default(""),
    bookedByLastName: text("booked_by_last_name").notNull().default(""),
    bookedByUserId: text("booked_by_user_id"),
    bookedForOther: boolean("booked_for_other").notNull().default(false),
    email: text("email"),
    subject: text("subject"),
    className: text("class_name"),
    comment: text("comment"),
    startsAt: text("starts_at").notNull(),
    endsAt: text("ends_at").notNull(),
    status: text("status").notNull().default("CONFIRMED"),
    cancelledAt: text("cancelled_at"),
    cancelledBy: text("cancelled_by"),
    cancelReason: text("cancel_reason"),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
  },
  (t) => [
    primaryKey({ columns: [t.etablissementId, t.id], name: "reservation_room_booking_pk" }),
    index("reservation_room_booking_etab_idx").on(t.etablissementId),
    index("reservation_room_booking_room_idx").on(t.etablissementId, t.roomId),
    index("reservation_room_booking_starts_idx").on(t.etablissementId, t.startsAt),
    index("reservation_room_booking_status_idx").on(t.etablissementId, t.status),
    index("reservation_room_booking_group_idx").on(t.etablissementId, t.groupId),
  ],
);

export const reservationRoomsSchema = {
  reservationRoom,
  reservationRoomBooking,
};
