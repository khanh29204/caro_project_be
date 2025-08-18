import { Server, Socket } from "socket.io";
import {
  rooms,
  makeRoom,
  checkWinner,
  incHistory,
  isBoardFull,
  newBoard,
  pickNewHost,
} from "../store";
import { JoinPayload, MovePayload, User, Presence } from "../types";

const OFFLINE_GRACE_MS = 30_000;

export function setupSocket(httpServer: any) {
  const io = new Server(httpServer, {
    path: "/api/socket.io",
    cors: { origin: true, credentials: true },
  });

  io.on("connection", (socket: Socket) => {
    socket.data.user = null as null | User;
    socket.data.roomId = null as null | string;

    // -------- join-room --------
    socket.on("join-room", (payload: JoinPayload) => {
      const { roomId, user } = payload;
      if (!user?.id) return;

      socket.data.user = user;
      socket.data.roomId = roomId;

      let room = rooms.get(roomId);
      if (!room) {
        room = makeRoom(roomId);
        rooms.set(roomId, room);
      }

      // clear offline timer nếu có
      const t = room.offlineTimers.get(user.id);
      if (t) {
        clearTimeout(t);
        room.offlineTimers.delete(user.id);
      }

      // cập nhật presence
      const presence: Presence = room.members.get(user.id) ?? {
        user,
        sockets: new Set<string>(),
      };
      presence.user = user; // update name nếu đổi
      presence.sockets.add(socket.id);
      room.members.set(user.id, presence);

      // push vào players nếu còn slot
      const already = room.players.find((p) => p.id === user.id);
      if (!already && room.players.length < 2) {
        const symbol = room.players.length === 0 ? "X" : "O";
        room.players.push({ ...user, symbol });
      }

      // host
      if (!room.hostId) room.hostId = user.id;

      socket.join(roomId);
      io.to(roomId).emit("room-state", room);
    });

    // -------- make-move --------
    socket.on("make-move", (payload: MovePayload) => {
      const user: User | null = socket.data.user;
      const roomId: string | null = socket.data.roomId;
      if (!user || !roomId) return;

      const room = rooms.get(roomId);
      if (!room || room.winner) return;

      const player = room.players.find((p) => p.id === user.id);
      if (!player) return;
      if (room.nextTurn !== player.symbol) return;

      const { x, y } = payload;
      const rows = room.board.length;
      const cols = room.board[0]?.length ?? 0;
      if (x < 0 || y < 0 || y >= rows || x >= cols) return;
      if (room.board[y][x] !== 0) return;

      room.board[y][x] = player.symbol === "X" ? 1 : -1;
      room.lastMove = { x, y };

      const win = checkWinner(room.board, x, y);
      if (win) {
        room.winner = win;
        const opp = room.players.find((p) => p.id !== player.id);
        incHistory(player.id, player.id, opp?.id ?? player.id);
      } else if (isBoardFull(room.board)) {
        room.winner = "draw";
        if (room.players.length === 2) {
          incHistory(null, room.players[0].id, room.players[1].id);
        }
      } else {
        room.nextTurn = room.nextTurn === "X" ? "O" : "X";
      }

      io.to(roomId).emit("room-state", room);
    });

    // -------- restart --------
    socket.on("restart", () => {
      const user: User | null = socket.data.user;
      const roomId: string | null = socket.data.roomId;
      if (!user || !roomId) return;

      const room = rooms.get(roomId);
      if (!room) return;

      const isPlayer = room.players.some((p) => p.id === user.id);
      if (!isPlayer) return;

      room.board = newBoard();
      room.winner = null;
      room.lastMove = undefined;
      room.nextTurn = "X";
      io.to(roomId).emit("room-state", room);
    });

    // -------- disconnect --------
    socket.on("disconnect", () => {
      const user: User | null = socket.data.user;
      const roomId: string | null = socket.data.roomId;
      if (!user || !roomId) return;

      const room = rooms.get(roomId);
      if (!room) return;

      socket.leave(roomId);

      const presence = room.members.get(user.id);
      if (presence) {
        presence.sockets.delete(socket.id);
        // còn tab khác của user → vẫn online
        if (presence.sockets.size > 0) {
          io.to(roomId).emit("room-state", room);
          return;
        }
      }

      // user thực sự offline
      if (!room.offlineTimers.has(user.id)) {
        const timer = setTimeout(() => {
          // dọn user
          room.members.delete(user.id);

          // host?
          if (room.hostId === user.id) {
            room.hostId = pickNewHost(room);
            if (!room.hostId) {
              rooms.delete(roomId);
              io.in(roomId).socketsLeave(roomId);
              io.emit("room-deleted", roomId);
              room.offlineTimers.delete(user.id);
              return;
            }
          }

          io.to(roomId).emit("room-state", room);
          room.offlineTimers.delete(user.id);
        }, OFFLINE_GRACE_MS);
        // không giữ process vì timer
        // @ts-ignore
        if (typeof (timer as any).unref === "function") (timer as any).unref();
        room.offlineTimers.set(user.id, timer);
      }

      io.to(roomId).emit("room-state", room);
    });
  });

  return io;
}
