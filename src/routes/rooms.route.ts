import { Router } from "express";
import { makeRoom, rooms } from "../store";

const router = Router();

// POST /api/rooms  -> { roomId }
router.post("/", (_req, res) => {
  const room = makeRoom();
  rooms.set(room.id, room);
  res.json({ roomId: room.id });
});

export default router;
