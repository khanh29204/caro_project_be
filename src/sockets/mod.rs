use socketioxide::extract::{Data, SocketRef};
use socketioxide::SocketIo;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::store::{self, AppState};
use crate::types::*;

const OFFLINE_GRACE_MS: u64 = 30_000;

/// Per-socket data stored in a shared map
struct SocketData {
    user: User,
    room_id: String,
}

lazy_static::lazy_static! {
    static ref SOCKET_DATA: RwLock<HashMap<String, SocketData>> = RwLock::new(HashMap::new());
}

fn get_socket_data(socket_id: &str) -> Option<(User, String)> {
    SOCKET_DATA.read().get(socket_id).map(|d| (d.user.clone(), d.room_id.clone()))
}

fn set_socket_data(socket_id: &str, user: User, room_id: String) {
    SOCKET_DATA.write().insert(socket_id.to_string(), SocketData { user, room_id });
}

fn remove_socket_data(socket_id: &str) {
    SOCKET_DATA.write().remove(socket_id);
}

/// Setup all Socket.IO event handlers (mirrors sockets/index.ts)
pub fn setup_socket(io: &SocketIo, app_state: Arc<AppState>) {
    let state = app_state.clone();
    let io_clone = io.clone();

    io.ns("/", move |socket: SocketRef| {
        let state = state.clone();
        let io_ref = io_clone.clone();
        tracing::info!("Socket connected: {}", socket.id);

        // -------- join-room --------
        {
            let state = state.clone();
            let io_join = io_ref.clone();
            socket.on("join-room", move |socket: SocketRef, Data::<serde_json::Value>(data)| {
                let payload: JoinPayload = match serde_json::from_value(data) {
                    Ok(p) => p,
                    Err(_) => return,
                };

                let JoinPayload { room_id, user } = payload;
                if user.id.is_empty() {
                    return;
                }

                // Store user and roomId per socket
                set_socket_data(&socket.id.to_string(), user.clone(), room_id.clone());

                let mut rooms = state.rooms.write();
                let room = rooms.entry(room_id.clone()).or_insert_with(|| {
                    store::make_room(Some(room_id.clone()))
                });

                // Clear offline timer if exists
                room.offline_timers.remove(&user.id);

                // Update presence
                let presence = room.members.entry(user.id.clone()).or_insert_with(|| {
                    Presence {
                        user: user.clone(),
                        sockets: HashSet::new(),
                    }
                });
                presence.user = user.clone(); // update name if changed
                presence.sockets.insert(socket.id.to_string());

                // Push into players if slot available
                let already = room.players.iter().any(|p| p.id == user.id);
                if !already {
                    if let Some(host_id) = &room.host_id {
                        if &user.id == host_id {
                            // Creator always gets X
                            if room.players.is_empty() {
                                room.players.push(Player { id: user.id.clone(), name: user.name.clone(), symbol: SymbolXO::X });
                            } else {
                                // Insert at the front so they are the first player
                                room.players.insert(0, Player { id: user.id.clone(), name: user.name.clone(), symbol: SymbolXO::X });
                            }
                        } else {
                            // Non-host joins
                            if !room.players.iter().any(|p| p.symbol == SymbolXO::O) {
                                room.players.push(Player { id: user.id.clone(), name: user.name.clone(), symbol: SymbolXO::O });
                            }
                        }
                    } else {
                        // Old logic if host_id is somehow missing
                        if room.players.len() < 2 {
                            let symbol = if room.players.is_empty() { SymbolXO::X } else { SymbolXO::O };
                            room.players.push(Player { id: user.id.clone(), name: user.name.clone(), symbol });
                        }
                    }
                }

                // Set host if not set
                if room.host_id.is_none() {
                    room.host_id = Some(user.id.clone());
                }

                // Set firstMoverId for X player
                if room.first_mover_id.is_none() {
                    if let Some(px) = room.players.iter().find(|p| p.symbol == SymbolXO::X) {
                        room.first_mover_id = Some(px.id.clone());
                    }
                }

                // Join socket room
                socket.leave_all().ok();
                socket.join(room_id.clone()).ok();

                let room_json = room.to_json();
                drop(rooms);

                // io.to() broadcasts to ALL in room (including sender)
                io_join.to(room_id).emit("room-state", &room_json).ok();
            });
        }

        // -------- make-move --------
        {
            let state = state.clone();
            let io_move = io_ref.clone();
            socket.on("make-move", move |socket: SocketRef, Data::<serde_json::Value>(data)| {
                let payload: MovePayload = match serde_json::from_value(data) {
                    Ok(p) => p,
                    Err(_) => return,
                };

                let (user, room_id) = match get_socket_data(&socket.id.to_string()) {
                    Some(d) => d,
                    None => return,
                };

                let mut rooms = state.rooms.write();
                let room = match rooms.get_mut(&room_id) {
                    Some(r) => r,
                    None => return,
                };

                if room.winner.is_some() {
                    return;
                }

                let player = match room.players.iter().find(|p| p.id == user.id) {
                    Some(p) => p.clone(),
                    None => return,
                };

                if room.next_turn != player.symbol {
                    return;
                }

                let MovePayload { x, y } = payload;
                let rows = room.board.len();
                let cols = if rows > 0 { room.board[0].len() } else { 0 };

                if y >= rows || x >= cols {
                    return;
                }
                if room.board[y][x] != 0 {
                    return;
                }

                room.board[y][x] = if player.symbol == SymbolXO::X { 1 } else { -1 };
                room.last_move = Some((x, y));

                let win = store::check_winner(&room.board, x, y);
                if let Some(winner_symbol) = win {
                    room.winner = Some(WinnerState::Symbol(winner_symbol));
                    room.last_winner_id = Some(player.id.clone());

                    let opp = room.players.iter().find(|p| p.id != player.id);
                    if let Some(opp) = opp {
                        state.db.inc_history(
                            Some(&player.id),
                            &player.id,
                            &opp.id,
                        );
                    }
                } else if store::is_board_full(&room.board) {
                    room.winner = Some(WinnerState::Draw);
                    room.last_winner_id = None;

                    if room.players.len() == 2 {
                        let p0 = room.players[0].id.clone();
                        let p1 = room.players[1].id.clone();
                        state.db.inc_history(None, &p0, &p1);
                    }
                } else {
                    room.next_turn = if room.next_turn == SymbolXO::X {
                        SymbolXO::O
                    } else {
                        SymbolXO::X
                    };
                }

                let room_json = room.to_json();
                drop(rooms);

                // io.to() broadcasts to ALL in room (including sender)
                io_move.to(room_id).emit("room-state", &room_json).ok();
            });
        }

        // -------- restart --------
        {
            let state = state.clone();
            let io_restart = io_ref.clone();
            socket.on("restart", move |socket: SocketRef| {
                let (user, room_id) = match get_socket_data(&socket.id.to_string()) {
                    Some(d) => d,
                    None => return,
                };

                let mut rooms = state.rooms.write();
                let room = match rooms.get_mut(&room_id) {
                    Some(r) => r,
                    None => return,
                };

                let is_player = room.players.iter().any(|p| p.id == user.id);
                if !is_player {
                    return;
                }

                // Xác định ai đi trước ván mới
                let starter_id = room.last_winner_id.clone().or_else(|| room.first_mover_id.clone());
                let starter_symbol = starter_id.as_ref().and_then(|id| store::symbol_of(room, id));

                room.board = store::new_board();
                room.winner = None;
                room.last_move = None;

                if let Some(sym) = starter_symbol {
                    room.next_turn = sym;
                    room.first_mover_id = starter_id;
                } else {
                    room.next_turn = SymbolXO::X;
                    let px = room.players.iter().find(|p| p.symbol == SymbolXO::X);
                    room.first_mover_id = px.map(|p| p.id.clone());
                }

                room.last_winner_id = None;

                let room_json = room.to_json();
                drop(rooms);

                // io.to() broadcasts to ALL in room (including sender)
                io_restart.to(room_id).emit("room-state", &room_json).ok();
            });
        }

        // -------- disconnect --------
        {
            let state = state.clone();
            let io_disconnect = io_ref.clone();
            socket.on_disconnect(move |socket: SocketRef| {
                let (user, room_id) = match get_socket_data(&socket.id.to_string()) {
                    Some(d) => d,
                    None => return,
                };

                remove_socket_data(&socket.id.to_string());

                tracing::info!("Socket disconnected: {} user: {}", socket.id, user.id);

                {
                    let mut rooms = state.rooms.write();
                    let room = match rooms.get_mut(&room_id) {
                        Some(r) => r,
                        None => return,
                    };

                    let has_presence = room.members.contains_key(&user.id);
                    if !has_presence {
                        let room_json = room.to_json();
                        drop(rooms);
                        io_disconnect.to(room_id).emit("room-state", &room_json).ok();
                        return;
                    }

                    let still_online = {
                        let presence = room.members.get_mut(&user.id).unwrap();
                        presence.sockets.remove(&socket.id.to_string());
                        !presence.sockets.is_empty()
                    };

                    if still_online {
                        let room_json = room.to_json();
                        drop(rooms);
                        io_disconnect.to(room_id).emit("room-state", &room_json).ok();
                        return;
                    }

                    if room.offline_timers.contains(&user.id) {
                        let room_json = room.to_json();
                        drop(rooms);
                        io_disconnect.to(room_id).emit("room-state", &room_json).ok();
                        return;
                    }

                    room.offline_timers.insert(user.id.clone());
                    let room_json = room.to_json();
                    drop(rooms);
                    io_disconnect.to(room_id.clone()).emit("room-state", &room_json).ok();
                }

                // Spawn offline grace timer (30s)
                let state = state.clone();
                let user_id = user.id.clone();
                let rid = room_id.clone();
                let io_timer = io_disconnect.clone();

                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(OFFLINE_GRACE_MS)).await;

                    let mut rooms = state.rooms.write();
                    let room = match rooms.get_mut(&rid) {
                        Some(r) => r,
                        None => return,
                    };

                    // Check if user reconnected
                    if let Some(prez) = room.members.get(&user_id) {
                        if !prez.sockets.is_empty() {
                            room.offline_timers.remove(&user_id);
                            return;
                        }
                    }

                    // Remove user
                    room.members.remove(&user_id);
                    room.players.retain(|p| p.id != user_id);

                    if room.host_id.as_deref() == Some(&user_id) {
                        room.host_id = store::pick_new_host(room);

                        if room.host_id.is_none() {
                            let rid_clone = rid.clone();
                            rooms.remove(&rid);
                            io_timer.to(rid_clone.clone()).emit("room-deleted", &rid_clone).ok();
                            return;
                        }
                    }

                    room.offline_timers.remove(&user_id);
                    let room_json = room.to_json();
                    drop(rooms);

                    io_timer.to(rid).emit("room-state", &room_json).ok();
                });
            });
        }
    });
}
