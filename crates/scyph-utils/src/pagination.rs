// crates/scyph-utils/src/pagination.rs
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub id: Uuid,
}

impl Cursor {
    pub fn encode(id: Uuid) -> String {
        URL_SAFE_NO_PAD.encode(id.as_bytes())
    }
    pub fn decode(s: &str) -> Option<Uuid> {
        let bytes = URL_SAFE_NO_PAD.decode(s).ok()?;
        Uuid::from_slice(&bytes).ok()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PageParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub cursor: Option<String>,
}
fn default_limit() -> i64 {
    20
}

impl PageParams {
    pub fn cursor_id(&self) -> Option<Uuid> {
        self.cursor.as_deref().and_then(Cursor::decode)
    }
}
