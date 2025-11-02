use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Event {
    pub event: i64,
    pub time: u64,
    pub actor: Option<String>,
    pub receptor: Option<String>,
    pub origin: Option<String>,
    pub destination: Option<String>,
    pub category: String,
    #[serde(default)]
    pub data: Vec<String>,
}
