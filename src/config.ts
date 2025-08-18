import dotenv from "dotenv";
dotenv.config();

export const PORT = Number(process.env.PORT || 3001);

export const ALLOWED_ORIGINS = (process.env.ALLOWED_ORIGINS || "")
  .split(",").map(s => s.trim()).filter(Boolean);

export const BOARD_SIZE = Number(process.env.BOARD_SIZE || 15);
