import { sqliteTable, text, integer } from "drizzle-orm/sqlite-core";

export const histories = sqliteTable("histories", {
  // Key dạng "userId1|userId2" (đã sort)
  pairKey: text("pair_key").primaryKey(), 
  
  playerA: text("player_a").notNull(),
  playerB: text("player_b").notNull(),
  
  winsA: integer("wins_a").default(0).notNull(),
  winsB: integer("wins_b").default(0).notNull(),
  draws: integer("draws").default(0).notNull(),
  
  updatedAt: integer("updated_at", { mode: "timestamp" }).$onUpdate(() => new Date()),
});