use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogContent {
    #[serde(rename = "fromToken")]
    pub from_token: String,
    #[serde(rename = "toToken")]
    pub to_token: String,
    #[serde(rename = "fromTokenAddr")]
    pub from_token_addr: String,
    #[serde(rename = "toTokenAddr")]
    pub to_token_addr: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "fromDecimal")]
    pub from_decimal: u16,
    #[serde(rename = "toDecimal")]
    pub to_decimal: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouterResult {
    pub code: u16,
    pub data: Option<Vec<Path>>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Path {
    #[serde(rename = "amount")]
    pub amount: String,
    #[serde(rename = "fee")]
    pub fee: String,
    #[serde(rename = "impact")]
    pub impact: String,
    #[serde(rename = "inUsd")]
    pub in_usd: String,
    #[serde(rename = "outUsd")]
    pub out_usd: String,
    #[serde(rename = "pool")]
    pub pool: Vec<String>,
    #[serde(rename = "roadForAddr")]
    pub road_for_addr: Vec<String>,
    #[serde(rename = "roadForName")]
    pub road_for_name: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "logFilePath")]
    pub log_file_path: String,
    #[serde(rename = "oldResPath")]
    pub old_res_path: String,
    #[serde(rename = "newResPath")]
    pub new_res_path: String,
    #[serde(rename = "compareResPath")]
    pub compare_res_path: String,
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let content =
            std::fs::read_to_string(path).expect("Unable to find the specified config file");
        serde_json::from_str(&content).expect("Invalid configuration file provided")
    }
}