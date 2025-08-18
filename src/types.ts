export type SymbolXO = "X" | "O";
export type Cell = 0 | 1 | -1; // 1 = X, -1 = O
export type Board = Cell[][];

export type User = { id: string; name: string };
export type Player = User & { symbol: SymbolXO };

export type Room = {
  id: string;
  board: Board;
  players: Player[];               // max 2
  nextTurn: SymbolXO;
  winner: null | SymbolXO | "draw";
  lastMove?: { x: number; y: number };
  hostId?: string;
  members: Map<string, User>;
  offlineTimers: Map<string, NodeJS.Timeout>;
};

export type PairKey = string; // "userA|userB" (sorted)
export type Versus = { a: string; b: string; winsA: number; winsB: number; draws: number };

export type JoinPayload = { roomId: string; user: User };
export type MovePayload = { x: number; y: number };
