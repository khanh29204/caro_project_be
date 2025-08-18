import { Router } from "express";
import { perspectiveHistory } from "../store";

const router = Router();

// GET /api/history?userId=&opponentId=
router.get("/", (req, res) => {
  const userId = String(req.query.userId || "");
  const opponentId = String(req.query.opponentId || "");
  if (!userId || !opponentId) return res.status(400).json({ error: "userId & opponentId required" });
  return res.json(perspectiveHistory(userId, opponentId));
});

export default router;
