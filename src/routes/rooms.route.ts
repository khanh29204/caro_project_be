import { Router } from "express";
import { makeRoom, rooms } from "../store";
import { getIO } from "../sockets";

const router = Router();

// POST /api/rooms  -> { roomId }
router.post("/", (_req, res) => {
  const room = makeRoom();
  rooms.set(room.id, room);
  res.json({ roomId: room.id });
});

router.delete("/:roomId", (req, res) => {
  const { roomId } = req.params;
  const room = rooms.get(roomId);
  if (!room) {
    return res.status(404).json({ error: "Room not found" });
  }
  //gửi message socket
  getIO().to(roomId).emit("room-deleted", roomId);
  rooms.delete(roomId);
  res.status(204).send();
});

export default router;
