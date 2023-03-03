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
    pub amount: Option<String>,
    #[serde(rename = "fee")]
    pub fee: Option<String>,
    #[serde(rename = "impact")]
    pub impact: Option<String>,
    #[serde(rename = "inUsd")]
    pub in_usd: Option<String>,
    #[serde(rename = "outUsd")]
    pub out_usd: Option<String>,
    #[serde(rename = "pool")]
    pub pool: Option<Vec<String>>,
    #[serde(rename = "roadForAddr")]
    pub road_for_addr: Option<Vec<String>>,
    #[serde(rename = "roadForName")]
    pub road_for_name: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "oldUrl")]
    pub old_url: String,
    #[serde(rename = "newUrl")]
    pub new_url: String,
    #[serde(rename = "logFilePath")]
    pub log_file_path: String,
    #[serde(rename = "oldResPath")]
    pub old_res_path: String,
    #[serde(rename = "newResPath")]
    pub new_res_path: String,
    #[serde(rename = "compareResPath")]
    pub compare_res_path: String,
    #[serde(rename = "useBaseTokens")]
    pub use_base_tokens: String,
    #[serde(rename = "maxCount")]
    pub max_count: u64,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompareResult {
    #[serde(rename = "OldAmount")]
    pub old_amount: String,
    #[serde(rename = "OldFee")]
    pub old_fee: String,
    #[serde(rename = "newAmount")]
    pub new_amount: String,
    #[serde(rename = "newFee")]
    pub new_fee: String,
    #[serde(rename = "diffAmount")]
    pub diff_amount: f64,
    #[serde(rename = "diffFee")]
    pub diff_fee: f64,
}

impl CompareResult {
    pub fn gen_from_paths(old: &Path, new: &Path) -> Self {
        let old_amount_f = old.amount.clone().unwrap().parse::<f64>().expect("parse to f64");
        let old_fee_f = old.fee.clone().unwrap().parse::<f64>().expect("parse to f64");
        let new_amount_f = new.clone().amount.unwrap().parse::<f64>().expect("parse to f64");
        let new_fee_f = new.clone().fee.unwrap().parse::<f64>().expect("parse to f64");

        Self {
            old_amount: old.clone().amount.unwrap().clone(),
            old_fee: old.clone().fee.unwrap().clone(),
            new_amount: new.clone().amount.unwrap().clone(),
            new_fee: new.clone().fee.unwrap().clone(),
            diff_amount: new_amount_f - old_amount_f,
            diff_fee: new_fee_f - old_fee_f,
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let content =
            std::fs::read_to_string(path).expect("Unable to find the specified config file");
        serde_json::from_str(&content).expect("Invalid configuration file provided")
    }
}