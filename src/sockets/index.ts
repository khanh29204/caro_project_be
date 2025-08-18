import { Server, Socket } from "socket.io";
import {
  rooms,
  makeRoom,
  checkWinner,
  incHistory,
  isBoardFull,
  newBoard,
  pickNewHost,
  symbolOf,
} from "../store";
import { JoinPayload, MovePayload, User, Presence, SymbolXO } from "../types";

const OFFLINE_GRACE_MS = 30000;
let io: Server;
export function setupSocket(httpServer: any) {
  io = new Server(httpServer, {
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
        const symbol: SymbolXO = room.players.length === 0 ? "X" : "O";
        room.players.push({ ...user, symbol });

        // nếu chưa có firstMoverId và người này là X → mặc định người X đi trước
        if (!room.firstMoverId && symbol === "X") {
          room.firstMoverId = user.id;
        }
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
        room.lastWinnerId = player.id;
        const opp = room.players.find((p) => p.id !== player.id);
        if (opp) {
          incHistory(player.id, player.id, opp.id);
        }
      } else if (isBoardFull(room.board)) {
        room.winner = "draw";
        room.lastWinnerId = null;
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

      // ✅ Xác định ai đi trước ván mới:
      // - nếu có người thắng ván trước → họ đi trước
      // - nếu hòa → giữ nguyên người đi trước ván trước (firstMoverId)
      const starterId = room.lastWinnerId ?? room.firstMoverId ?? null;
      const starterSymbol = starterId ? symbolOf(room, starterId) : null;

      room.board = newBoard();
      room.winner = null;
      room.lastMove = undefined;

      if (starterSymbol) {
        room.nextTurn = starterSymbol; // người thắng (hoặc người cũ) đi trước
        room.firstMoverId = starterId; // lưu cho ván sau (nếu tiếp tục hòa)
      } else {
        // fallback an toàn nếu thiếu dữ liệu
        room.nextTurn = "X";
        // ưu tiên người mang X nếu có
        const px = room.players.find((p) => p.symbol === "X");
        room.firstMoverId = px?.id ?? null;
      }

      room.lastWinnerId = null; // reset trạng thái winner của ván mới

      io.to(roomId).emit("room-state", room);
    });

    // -------- disconnect --------
    socket.on("disconnect", (reason) => {
      const user: User | null = socket.data.user;
      const roomId: string | null = socket.data.roomId;
      if (!user || !roomId) return;

      const room = rooms.get(roomId);
      if (!room) return;

      socket.leave(roomId);

      const presence = room.members.get(user.id);
      if (!presence) {
        // Không có presence -> có thể đã bị dọn ở đâu đó, đừng set timer nữa
        io.to(roomId).emit("room-state", room);
        return;
      }

      // Gỡ socket này khỏi danh sách của user
      presence.sockets.delete(socket.id);

      // Nếu user vẫn còn socket khác -> vẫn online, chỉ cần broadcast là xong
      if (presence.sockets.size > 0) {
        io.to(roomId).emit("room-state", room);
        return;
      }

      // Tới đây: user thật sự offline (không còn socket nào)
      // Tránh double timer: nếu đã có timer đang chờ thì thôi
      if (room.offlineTimers.has(user.id)) {
        // đã có timer giữ ghế -> chỉ broadcast một lần cho FE biết 'offline'
        io.to(roomId).emit("room-state", room);
        return;
      }

      // Đặt timer giữ ghế 30s
      const timer = setTimeout(() => {
        // ❗ Re-check vì trong 30s user có thể đã quay lại
        const r = rooms.get(roomId);
        if (!r) return; // phòng đã xoá ở nơi khác

        const prez = r.members.get(user.id);
        if (prez && prez.sockets.size > 0) {
          // user đã quay lại -> không dọn nữa
          r.offlineTimers.delete(user.id);
          return;
        }

        // Dọn user khỏi 'members'
        r.members.delete(user.id);

        // (Optional) nếu muốn nhả ghế player sau khi rời hẳn:
        r.players = r.players.filter((p) => p.id !== user.id);

        // Nếu là host -> chọn host mới
        if (r.hostId === user.id) {
          r.hostId = pickNewHost(r);

          // Không còn ai -> xoá phòng
          if (!r.hostId) {
            rooms.delete(roomId);
            io.in(roomId).socketsLeave(roomId);
            io.emit("room-deleted", roomId);
            r.offlineTimers.delete(user.id);
            return; // Đừng emit room-state sau khi xoá
          }
        }

        io.to(roomId).emit("room-state", r);
        r.offlineTimers.delete(user.id);
      }, OFFLINE_GRACE_MS);

      // Đừng giữ process sống chỉ vì timer này
      // @ts-ignore
      if (typeof (timer as any).unref === "function") (timer as any).unref();

      room.offlineTimers.set(user.id, timer);

      // Emit 1 lần để FE biết user vừa rớt (offline state)
      io.to(roomId).emit("room-state", room);
    });
  });

  return io;
}
export function getIO() {
  if (!io)
    throw new Error("Socket.io chưa được khởi tạo, hãy gọi setupSocket trước!");
  return io;
}
