use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use anyhow::format_err;
use log::info;
use serde_json::Value;
use types::{LogContent, RouterResult};

mod utils;
mod client;
mod types;

pub use client::client::RouterApiClient;
pub use types::Config;
pub use utils::init_log;

const SWAP_ROUTING_FLAG: &str = "request-swap-routingInV2";
const GRAFANA_INFO_FLAG: &str = "--GRAFANA_INFO--";
const LOG_CONTENT_FLAG: &str = "logContent";

pub async fn parse_logs_fn(client: &mut RouterApiClient, config: Config) -> anyhow::Result<()> {
    let file = File::open(config.log_file_path.as_str())?;
    let reader = BufReader::new(file);
    let (mut old_file, mut new_file, mut compare_file) = get_output_files(config);
    let mut index: u64 = 0;
    for line in reader.lines() {
        let line_content = line?;
        if !line_content.contains(SWAP_ROUTING_FLAG) {
            continue;
        }
        let log_content = decode_to_log_content(&line_content)?;
        let old_res = client.call_old_router(&log_content).await?;
        let new_res = client.call_new_router(&log_content).await?;
        save_results(index, &old_res, &new_res, &mut old_file, &mut new_file, &mut compare_file);
        index += 1;
    }
    Ok(())
}

fn decode_to_log_content(line: &str) -> anyhow::Result<LogContent> {
    info!("Decode log {}", line);
    let res: Vec<_> = line.split(GRAFANA_INFO_FLAG).collect();
    if res.len() < 2 {
        return Err(anyhow::Error::msg("Not contain --GRAFANA_INFO-- "));
    }
    let a: Value = serde_json::from_str(res[1])?;
    if let Value::Object(map) = a {
        let log_content_str = map.get(LOG_CONTENT_FLAG)
            .ok_or(format_err!("fail to get logContent"))?
            .as_str().ok_or(format_err!("fail to parse logContent to str "))?;
        let log_content: LogContent = serde_json::from_str(log_content_str).unwrap();
        info!("Decode result: log_content:{:?}", log_content);
        return Ok(log_content);
    }
    return Err(format_err!("Fail to parse into json"));
}

fn get_output_files(config: Config) -> (File, File, File) {
    let old = OpenOptions::new().create(true).write(true).append(true).open(config.old_res_path.as_str()).unwrap();
    let new = OpenOptions::new().create(true).write(true).append(true).open(config.new_res_path.as_str()).unwrap();
    let compare = OpenOptions::new().create(true).write(true).append(true).open(config.compare_res_path.as_str()).unwrap();
    (old, new, compare)
}

fn save_results(
    index: u64,
    old: &RouterResult,
    new: &RouterResult,
    old_res: &mut File,
    new_res: &mut File,
    compare_res: &mut File) {
    let old_str = serde_json::to_string(old).unwrap();
    let new_str = serde_json::to_string(new).unwrap();
    let _ = old_res.write_all(format!("{}:{}\n", index, old_str).as_bytes());
    let _ = new_res.write_all(format!("{}:{}\n", index, new_str).as_bytes());
    if old.data.is_some() && new.data.is_some() {
        let item_old = old.data.as_ref().unwrap().get(0).unwrap();
        let item_new = new.data.as_ref().unwrap().get(0).unwrap();
        let compare_str = format!(
            "{} old:{}/{},new:{}/{}\n",
            index,
            item_old.amount,
            item_old.fee,
            item_new.amount,
            item_new.fee
        );
        let _ = compare_res.write_all(compare_str.as_bytes());
    }
}

