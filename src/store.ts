import { nanoid } from "nanoid";
import { BOARD_SIZE } from "./config";
import { Board, Room, SymbolXO, PairKey, Versus, Presence } from "./types";

export const rooms = new Map<string, Room>();
export const histories = new Map<PairKey, Versus>();

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

export function pairKey(id1: string, id2: string): PairKey {
  return [id1, id2].sort().join("|");
}

export function incHistory(winnerId: string | null, id1: string, id2: string) {
  // Chuẩn hoá cặp người chơi theo thứ tự cố định
  const [A, B] = [id1, id2].sort();
  if (A === B) return; // không ghi lịch sử tự đấu với chính mình

  const key = `${A}|${B}`;

  // Lấy hoặc tạo bản ghi canon
  const cur = histories.get(key) || {
    a: A,
    b: B,
    winsA: 0,
    winsB: 0,
    draws: 0,
  };

  // (Migration an toàn) Nếu a/b trong bản ghi cũ không khớp A/B, hoán đổi về đúng phía
  if (cur.a !== A || cur.b !== B) {
    // hoán đổi nếu cần
    if (cur.a === B && cur.b === A) {
      const tmp = cur.winsA;
      cur.winsA = cur.winsB;
      cur.winsB = tmp;
      cur.a = A;
      cur.b = B;
    } else {
      // trường hợp lạ: ép về canon
      cur.a = A;
      cur.b = B;
    }
  }

  // Cộng theo phía canon
  if (winnerId == null) {
    cur.draws++;
  } else if (winnerId === cur.a) {
    cur.winsA++;
  } else if (winnerId === cur.b) {
    cur.winsB++;
  }

  histories.set(key, cur);
}

export function perspectiveHistory(me: string, other: string) {
  const key = pairKey(me, other);
  const h = histories.get(key) || {
    a: me,
    b: other,
    winsA: 0,
    winsB: 0,
    draws: 0,
  };
  const meIsA = h.a === me;
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
