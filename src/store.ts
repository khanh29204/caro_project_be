import { nanoid } from "nanoid";
import { BOARD_SIZE } from "./config";
import { Board, Room, SymbolXO, Presence } from "./types";
import { db } from "./db";
import { histories } from "./db/schema";
import { eq, sql } from "drizzle-orm";

export const rooms = new Map<string, Room>();
// export const histories = new Map<PairKey, Versus>(); // -> Đã bỏ, dùng DB

export function newBoard(): Board {
  return Array.from({ length: BOARD_SIZE }, () =>
    Array(BOARD_SIZE).fill(0 as 0)
  );
}

export function makeRoom(id?: string): Room {
  return {
    id: id || nanoid(6).toUpperCase(),
    board: newBoard(),
    players: [],
    nextTurn: "X",
    winner: null,
    members: new Map<string, Presence>(),
    offlineTimers: new Map(),
    firstMoverId: null,
    lastWinnerId: null,
  };
}

export function symbolOf(room: Room, userId: string): SymbolXO | null {
  const p = room.players.find((p) => p.id === userId);
  return p ? p.symbol : null;
}

export function pairKey(id1: string, id2: string): string {
  return [id1, id2].sort().join("|");
}

// Chuyển sang async để gọi DB
export async function incHistory(
  winnerId: string | null,
  id1: string,
  id2: string
) {
  const [A, B] = [id1, id2].sort();
  if (A === B) return; // không ghi lịch sử tự đấu với chính mình

  const pKey = `${A}|${B}`;

  // Chuẩn bị data update
  let updateData: any = {};
  if (winnerId === null) {
    updateData = { draws: sql`draws + 1` };
  } else if (winnerId === A) {
    updateData = { winsA: sql`wins_a + 1` };
  } else {
    updateData = { winsB: sql`wins_b + 1` };
  }

  // Upsert (Insert nếu chưa có, Update nếu có rồi)
  await db
    .insert(histories)
    .values({
      pairKey: pKey,
      playerA: A,
      playerB: B,
      winsA: winnerId === A ? 1 : 0,
      winsB: winnerId === B ? 1 : 0,
      draws: winnerId === null ? 1 : 0,
    })
    .onConflictDoUpdate({
      target: histories.pairKey,
      set: updateData,
    });
}

const historyQuery = db
  .select()
  .from(histories)
  .where(eq(histories.pairKey, sql.placeholder("pKey")))
  .prepare();

export function perspectiveHistory(me: string, other: string) {
  const pKey = pairKey(me, other);
  const h = historyQuery.get({ pKey }) || {
    playerA: me < other ? me : other,
    playerB: me < other ? other : me,
    winsA: 0,
    winsB: 0,
    draws: 0,
  };

  const meIsA = h.playerA === me;
  return {
    me,
    opponent: other,
    wins: meIsA ? h.winsA : h.winsB,
    losses: meIsA ? h.winsB : h.winsA,
    draws: h.draws,
    total: h.winsA + h.winsB + h.draws,
  };
}

export function pickNewHost(room: Room): string | undefined {
  // 1) ưu tiên player còn online
  for (const p of room.players) {
    const prez = room.members.get(p.id);
    if (prez && prez.sockets.size > 0) return p.id;
  }
  // 2) khán giả còn online
  for (const [uid, prez] of room.members) {
    if (prez.sockets.size > 0) return uid;
  }
  return undefined;
}

export function checkWinner(
  board: Board,
  x: number,
  y: number
): SymbolXO | null {
  const val = board[y][x]; // 1 or -1
  if (!val) return null;
  const dirs: Array<[number, number]> = [
    [1, 0],
    [0, 1],
    [1, 1],
    [1, -1],
  ];
  for (const [dx, dy] of dirs) {
    let count = 1,
      i = 1;
    while (true) {
      const nx = x + dx * i,
        ny = y + dy * i;
      if (nx < 0 || ny < 0 || nx >= BOARD_SIZE || ny >= BOARD_SIZE) break;
      if (board[ny][nx] !== val) break;
      count++;
      i++;
    }
    i = 1;
    while (true) {
      const nx = x - dx * i,
        ny = y - dy * i;
      if (nx < 0 || ny < 0 || nx >= BOARD_SIZE || ny >= BOARD_SIZE) break;
      if (board[ny][nx] !== val) break;
      count++;
      i++;
    }
    if (count >= 5) return val === 1 ? "X" : "O";
  }
  return null;
}

export function isBoardFull(board: Board) {
  for (let y = 0; y < BOARD_SIZE; y++)
    for (let x = 0; x < BOARD_SIZE; x++) if (board[y][x] === 0) return false;
  return true;
}
