export type SymbolXO = "X" | "O";
export type Cell = 0 | 1 | -1; // 1 = X, -1 = O
export type Board = Cell[][];

export type User = { id: string; name: string };
export type Player = User & { symbol: SymbolXO };
export type Presence = { user: User; sockets: Set<string> };

export type Room = {
  id: string;
  board: Cell[][];
  players: Player[]; // max 2
  nextTurn: SymbolXO;
  winner: null | SymbolXO | "draw";
  lastMove?: { x: number; y: number };
  hostId?: string;

  // 🔥 CHUẨN HOÁ: presence map
  members: Map<string, Presence>; // userId -> { user, sockets }
  offlineTimers: Map<string, NodeJS.Timeout>;
  firstMoverId?: string | null; // ai đi trước ván hiện tại/tiếp theo
  lastWinnerId?: string | null;
};

export type PairKey = string; // "userA|userB" (sorted)
export type Versus = {
  a: string;
  b: string;
  winsA: number;
  winsB: number;
  draws: number;
};

export type JoinPayload = { roomId: string; user: User };
export type MovePayload = { x: number; y: number };
