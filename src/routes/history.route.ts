import { Router } from "express";
import { perspectiveHistory } from "../store";

const router = Router();

// GET /api/history?userId=&opponentId=
router.get("/", async (req, res) => {
  const userId = String(req.query.userId || "");
  const opponentId = String(req.query.opponentId || "");
  if (!userId || !opponentId) return res.status(400).json({ error: "userId & opponentId required" });
  
  try {
    const data = await perspectiveHistory(userId, opponentId);
    return res.json(data);
  } catch (error) {
    console.error(error);
    return res.status(500).json({ error: "Internal Error" });
  }
});

export default router;