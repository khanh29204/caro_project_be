import express from "express";
import cors from "cors";
import { ALLOWED_ORIGINS } from "./config";
import usersRoute from "./routes/users.route";
import roomsRoute from "./routes/rooms.route";
import historyRoute from "./routes/history.route";

export const app = express();

app.use(cors({
  origin: (origin, cb) => cb(null, true),
  credentials: true,
}));
app.use(express.json());

// health
app.get("/health", (_req, res) => res.json({ ok: true }));

// routes
app.use("/api/users", usersRoute);
app.use("/api/rooms", roomsRoute);
app.use("/api/history", historyRoute);
