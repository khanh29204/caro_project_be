use crate::config::board_size;
use crate::db::Database;
use crate::types::*;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};

/// Shared application state (mirrors rooms Map + store functions)
pub struct AppState {
    pub rooms: RwLock<HashMap<String, Room>>,
    pub db: Database,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        AppState {
            rooms: RwLock::new(HashMap::new()),
            db,
        }
    }
}

/// Create a new empty board (mirrors newBoard)
pub fn new_board() -> Board {
    let size = board_size();
    vec![vec![0i8; size]; size]
}

/// Create a new room (mirrors makeRoom)
pub fn make_room(id: Option<String>) -> Room {
    let room_id = id.unwrap_or_else(|| {
        nanoid::nanoid!(6, &nanoid::alphabet::SAFE).to_uppercase()
    });

    Room {
        id: room_id,
        board: new_board(),
        players: Vec::new(),
        next_turn: SymbolXO::X,
        winner: None,
        last_move: None,
        host_id: None,
        members: HashMap::new(),
        offline_timers: HashSet::new(),
        first_mover_id: None,
        last_winner_id: None,
    }
}

/// Get symbol of a player in the room (mirrors symbolOf)
pub fn symbol_of(room: &Room, user_id: &str) -> Option<SymbolXO> {
    room.players.iter()
        .find(|p| p.id == user_id)
        .map(|p| p.symbol)
}

/// Generate pair key from two user IDs (mirrors pairKey)
pub fn pair_key(id1: &str, id2: &str) -> String {
    let mut ids = [id1, id2];
    ids.sort();
    format!("{}|{}", ids[0], ids[1])
}

/// Pick a new host when current host disconnects (mirrors pickNewHost)
pub fn pick_new_host(room: &Room) -> Option<String> {
    // 1) ưu tiên player còn online
    for p in &room.players {
        if let Some(prez) = room.members.get(&p.id) {
            if !prez.sockets.is_empty() {
                return Some(p.id.clone());
            }
        }
    }
    // 2) khán giả còn online
    for (uid, prez) in &room.members {
        if !prez.sockets.is_empty() {
            return Some(uid.clone());
        }
    }
    None
}

/// Check if there's a winner after a move (mirrors checkWinner)
pub fn check_winner(board: &Board, x: usize, y: usize) -> Option<SymbolXO> {
    let size = board_size();
    let val = board[y][x];
    if val == 0 {
        return None;
    }

    let dirs: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

    for (dx, dy) in &dirs {
        let mut count = 1u32;

        // Forward direction
        let mut i: i32 = 1;
        loop {
            let nx = x as i32 + dx * i;
            let ny = y as i32 + dy * i;
            if nx < 0 || ny < 0 || nx >= size as i32 || ny >= size as i32 {
                break;
            }
            if board[ny as usize][nx as usize] != val {
                break;
            }
            count += 1;
            i += 1;
        }

        // Backward direction
        i = 1;
        loop {
            let nx = x as i32 - dx * i;
            let ny = y as i32 - dy * i;
            if nx < 0 || ny < 0 || nx >= size as i32 || ny >= size as i32 {
                break;
            }
            if board[ny as usize][nx as usize] != val {
                break;
            }
            count += 1;
            i += 1;
        }

        if count >= 5 {
            return if val == 1 {
                Some(SymbolXO::X)
            } else {
                Some(SymbolXO::O)
            };
        }
    }

    None
}

/// Check if the board is full (mirrors isBoardFull)
pub fn is_board_full(board: &Board) -> bool {
    let size = board_size();
    for y in 0..size {
        for x in 0..size {
            if board[y][x] == 0 {
                return false;
            }
        }
    }
    true
}
