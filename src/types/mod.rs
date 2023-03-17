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
    // #[serde(default)]
    // pub cast: u32,
    // #[serde(default)]
    // pub paths:u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "oldUrl")]
    pub old_url: String,
    #[serde(rename = "newUrl")]
    pub new_url: String,
    #[serde(rename = "logFilePath")]
    pub log_file_path: String,
    #[serde(rename = "compareResDetailPath")]
    pub compare_res_detail_path: String,
    #[serde(rename = "compareResPath")]
    pub compare_res_path: String,
    #[serde(rename = "useBaseTokens")]
    pub use_base_tokens: String,
    #[serde(rename = "maxCount")]
    pub max_count: u64,
    #[serde(rename = "restoreInput")]
    pub restore_input: bool,
    #[serde(rename = "restoreInputPath")]
    pub restore_input_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompareResult {
    #[serde(rename = "diffFeePer")]
    pub diff_fee_per: f64,
    #[serde(rename = "diffAmountPer")]
    pub diff_amount_per: f64,
    #[serde(rename = "diffImpactPer")]
    pub diff_impact_per: f64,
    #[serde(rename = "diffInusdPer")]
    pub diff_inusd_per: f64,
    #[serde(rename = "diffoutusdPer")]
    pub diff_outusd_per: f64,
    #[serde(rename = "poolEq")]
    pub pool_eq: bool,
    #[serde(rename = "roadForAddrEq")]
    pub road_addr_eq: bool,
}


impl CompareResult {
    pub fn gen_from_paths(old: &Path, new: &Path) -> Option<Self> {
        if old.amount.is_none() || new.amount.is_none() {
            return None;
        }
        let diff_amount_per = clac_string_per(old.amount.clone().unwrap(), new.amount.clone().unwrap());
        let diff_fee_per = clac_string_per(old.fee.clone().unwrap(), new.fee.clone().unwrap());
        let diff_impact_per = clac_string_per(old.impact.clone().unwrap(), new.impact.clone().unwrap());
        let diff_inusd_per = clac_string_per(old.in_usd.clone().unwrap(), new.in_usd.clone().unwrap());
        let diff_outusd_per = clac_string_per(old.out_usd.clone().unwrap(), new.out_usd.clone().unwrap());
        let pool_eq = old.pool.clone().unwrap().eq(&new.pool.clone().unwrap());
        let road_addr_eq = old.road_for_addr.clone().unwrap().eq(&new.road_for_addr.clone().unwrap());
        Some(
            Self {
                diff_fee_per,
                diff_amount_per,
                diff_impact_per,
                diff_inusd_per,
                diff_outusd_per,
                pool_eq,
                road_addr_eq,
            }
        )
    }
}


fn clac_string_per(a: String, b: String) -> f64 {
    let a_f = a.parse::<f64>().expect("parse to f6");
    let b_f = b.parse::<f64>().expect("parse to f6");
    clac_per(a_f.abs(), b_f.abs())
}

fn clac_per(a: f64, b: f64) -> f64 {
    ((a - b) / a).abs()
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        let content =
            std::fs::read_to_string(path).expect("Unable to find the specified config file");
        serde_json::from_str(&content).expect("Invalid configuration file provided")
    }
}