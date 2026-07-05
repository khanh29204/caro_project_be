use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// "X" or "O"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolXO {
    X,
    O,
}

impl std::fmt::Display for SymbolXO {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolXO::X => write!(f, "X"),
            SymbolXO::O => write!(f, "O"),
        }
    }
}

/// Cell value: 0 = empty, 1 = X, -1 = O
pub type Cell = i8;
pub type Board = Vec<Vec<Cell>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub symbol: SymbolXO,
}

/// Winner state: X, O, draw, or null (ongoing)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WinnerState {
    Symbol(SymbolXO),
    Draw,
}

// Custom serialization for WinnerState to match JS behavior
impl WinnerState {
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            WinnerState::Symbol(s) => serde_json::Value::String(s.to_string()),
            WinnerState::Draw => serde_json::Value::String("draw".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Presence {
    pub user: User,
    pub sockets: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct Room {
    pub id: String,
    pub board: Board,
    pub players: Vec<Player>,       // max 2
    pub next_turn: SymbolXO,
    pub winner: Option<WinnerState>,
    pub last_move: Option<(usize, usize)>, // (x, y)
    pub host_id: Option<String>,
    pub members: HashMap<String, Presence>, // userId -> Presence
    pub offline_timers: HashSet<String>,     // userId set (timers tracked externally)
    pub first_mover_id: Option<String>,
    pub last_winner_id: Option<String>,
}

/// Serialize Room to JSON matching the JS format exactly
impl Room {
    pub fn to_json(&self) -> serde_json::Value {
        let members: serde_json::Value = self.members.iter().map(|(uid, prez)| {
            (uid.clone(), serde_json::json!({
                "user": { "id": prez.user.id, "name": prez.user.name },
                "sockets": prez.sockets.iter().collect::<Vec<_>>(),
            }))
        }).collect::<serde_json::Map<String, serde_json::Value>>().into();

        let mut obj = serde_json::json!({
            "id": self.id,
            "board": self.board,
            "players": self.players,
            "nextTurn": self.next_turn,
            "winner": self.winner.as_ref().map(|w| w.to_json_value()),
            "members": members,
            "offlineTimers": {},
            "firstMoverId": self.first_mover_id,
            "lastWinnerId": self.last_winner_id,
        });

        if let Some((x, y)) = self.last_move {
            obj["lastMove"] = serde_json::json!({"x": x, "y": y});
        }

        if let Some(ref host_id) = self.host_id {
            obj["hostId"] = serde_json::json!(host_id);
        }

        obj
    }
}

#[derive(Debug, Deserialize)]
pub struct JoinPayload {
    #[serde(rename = "roomId")]
    pub room_id: String,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct MovePayload {
    pub x: usize,
    pub y: usize,
}
