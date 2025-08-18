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
import { JoinPayload, MovePayload, User } from "../types";
import { ALLOWED_ORIGINS } from "../config";

export function setupSocket(httpServer: any) {
  const io = new Server(httpServer, {
    path: "/api/socket.io",
    cors: { origin: ALLOWED_ORIGINS, credentials: true },
  });

  io.on("connection", (socket: Socket) => {
    socket.data.user = null as null | User;
    socket.data.roomId = null as null | string;

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

      const t = room.offlineTimers.get(user.id);
      if (t) {
        clearTimeout(t);
        room.offlineTimers.delete(user.id);
      }

      room.members.set(user.id, user);

      const already = room.players.find((p) => p.id === user.id);
      if (!already && room.players.length < 2) {
        const symbol = room.players.length === 0 ? "X" : "O";
        room.players.push({ ...user, symbol });
      }

      if (!room.hostId) room.hostId = user.id;

      socket.join(roomId);
      io.to(roomId).emit("room-state", room);
    });

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
      if (x < 0 || y < 0 || y >= room.board.length || x >= room.board[0].length)
        return;
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
        if (room.players.length === 2)
          incHistory(null, room.players[0].id, room.players[1].id);
      } else {
        room.nextTurn = room.nextTurn === "X" ? "O" : "X";
      }

      io.to(roomId).emit("room-state", room);
    });

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

    socket.on("disconnect", () => {
      const user: User | null = socket.data.user;
      const roomId: string | null = socket.data.roomId;
      if (!user || !roomId) return;

      const room = rooms.get(roomId);
      if (!room) return;

      if (!room.offlineTimers.has(user.id)) {
        const timer = setTimeout(() => {
          room.members.delete(user.id);
          if (room.hostId === user.id) {
            room.hostId = pickNewHost(room);
          }
          io.to(roomId).emit("room-state", room);
          room.offlineTimers.delete(user.id);
        }, 30_000);
        room.offlineTimers.set(user.id, timer);
      }

      io.to(roomId).emit("room-state", room);
    });
  });

  return io;
}
