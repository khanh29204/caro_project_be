import { Router } from "express";
import { nanoid } from "nanoid";

const router = Router();

// POST /api/users  -> { id, name }
router.post("/", (req, res) => {
  const { name } = req.body as { name?: string };
  if (!name || !name.trim()) return res.status(400).json({ error: "Name required" });
  const id = nanoid(12);
  res.json({ id, name: name.trim() });
});

export default router;
