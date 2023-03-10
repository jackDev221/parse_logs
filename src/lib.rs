use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use anyhow::format_err;
use log::{info, warn};
use serde_json::Value;

mod utils;
mod client;
mod types;

pub use client::client::RouterApiClient;
pub use types::{Config, LogContent, RouterResult, CompareResult};
pub use utils::init_log;

const SWAP_ROUTING_FLAG: &str = "request-swap-routingInV2";
const GRAFANA_INFO_FLAG: &str = "--GRAFANA_INFO--";
const LOG_CONTENT_FLAG: &str = "logContent";

pub async fn parse_logs_fn(client: &mut RouterApiClient, config: Config) -> anyhow::Result<()> {
    let file = File::open(config.log_file_path.as_str())?;
    let reader = BufReader::new(file);
    let (mut old_file, mut new_file, mut compare_detail_file, mut compare_file) = get_output_files(&config);
    let mut index: u64 = 0;
    let mut pos: u64 = 0;
    let mut neg: u64 = 0;
    let mut diff_pers: Vec<f64> = vec![0.0; 8];
    let _ = compare_detail_file.write_all("-------------------------Detail-----------------------\n".as_bytes());
    for line in reader.lines() {
        if index >= config.max_count {
            break;
        }
        let line_content = line?;
        if !line_content.contains(SWAP_ROUTING_FLAG) {
            continue;
        }
        let log_content = decode_to_log_content(&line_content)?;
        if let Ok((old_res, new_res)) = call_router_servers(client, &log_content).await {
            let (res, diff_amount, _, diff_amount_per) = save_results(
                index,
                &old_res,
                &new_res,
                &mut old_file,
                &mut new_file,
                &mut compare_detail_file,
            );
            if res {
                calc_compare_res(&mut diff_pers, diff_amount_per);
                if diff_amount >= 0.0 {
                    pos += 1;
                } else {
                    neg += 1;
                }
            }
            index += 1;
        } else {
            warn!("Fail to get response for {}", line_content);
        }
    }
    let _ = compare_file.write_all("\n-------------------------Result-----------------------\n".as_bytes());
    let compare_res = format!(
        "Result: pos:{}, neg:{}\n",
        pos,
        neg);
    let _ = compare_file.write_all(compare_res.as_bytes());
    let _ = compare_file.write_all(compare_res_to_string(&mut diff_pers).as_bytes());
    Ok(())
}


fn calc_compare_res(diff_pers: &mut Vec<f64>, diff_per: f64) {
    if diff_per < 0.0001 {
        diff_pers[0] += 1.0;
        return;
    }
    if 0.0001 <= diff_per && diff_per < 0.001 {
        diff_pers[1] += 1.0;
        return;
    }
    if 0.001 <= diff_per && diff_per < 0.01 {
        diff_pers[2] += 1.0;
        return;
    }
    if 0.01 <= diff_per && diff_per < 0.02 {
        diff_pers[3] += 1.0;
        return;
    }
    if 0.02 <= diff_per && diff_per < 0.05 {
        diff_pers[4] += 1.0;
        return;
    }

    if 0.05 <= diff_per && diff_per < 0.1 {
        diff_pers[5] += 1.0;
        return;
    }
    diff_pers[6] += 1.0;
}

fn compare_res_to_string(diff_pers: &mut Vec<f64>) -> String {
    let count: f64 = diff_pers.iter().sum();
    diff_pers[0] /= count;
    diff_pers[1] /= count;
    diff_pers[2] /= count;
    diff_pers[3] /= count;
    diff_pers[4] /= count;
    diff_pers[5] /= count;
    diff_pers[6] /= count;
    diff_pers[7] /= count;

    format!(
        "count:{}, diff <0.01%: {},  0.01%~0.1%:{}, 0.1%~1%:{}, 1%~2%:{}, 2%~5%:{}, 5%~10%:{}, >10%:{}",
        count,
        diff_pers[0] * 100,
        diff_pers[2] * 100,
        diff_pers[3] * 100,
        diff_pers[4] * 100,
        diff_pers[5] * 100,
        diff_pers[6] * 100,
        diff_pers[7] * 100,
    )
}


async fn call_router_servers(
    client: &mut RouterApiClient,
    log_content: &LogContent,
) -> anyhow::Result<(RouterResult, RouterResult)> {
    let old_res = client.call_old_router(log_content).await?;
    let new_res = client.call_new_router(log_content).await?;
    Ok((old_res, new_res))
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

fn get_output_files(config: &Config) -> (File, File, File, File) {
    let old = OpenOptions::new().create(true).write(true).append(true).open(config.old_res_path.as_str()).unwrap();
    let new = OpenOptions::new().create(true).write(true).append(true).open(config.new_res_path.as_str()).unwrap();
    let compare_detail = OpenOptions::new().create(true).write(true).append(true).open(config.compare_res_detail_path.as_str()).unwrap();
    let compare = OpenOptions::new().create(true).write(true).append(true).open(config.compare_res_path.as_str()).unwrap();
    (old, new, compare_detail, compare)
}

fn save_results(
    index: u64,
    old: &RouterResult,
    new: &RouterResult,
    old_res: &mut File,
    new_res: &mut File,
    compare_res: &mut File) -> (bool, f64, f64, f64) {
    let item_old = old.data.as_ref().unwrap().get(0).unwrap();
    let item_new = new.data.as_ref().unwrap().get(0).unwrap();
    if item_old.amount.is_none() {
        return (false, 0.0, 0.0, 0.0);
    }
    let old_str = serde_json::to_string(old).unwrap();
    let new_str = serde_json::to_string(new).unwrap();
    let _ = old_res.write_all(format!("{}: {}\n", index, old_str).as_bytes());
    let _ = new_res.write_all(format!("{}: {}\n", index, new_str).as_bytes());
    let compare = CompareResult::gen_from_paths(item_old, item_new);

    let compare_str = format!(
        "{}: {}\n",
        index,
        serde_json::to_string(&compare).expect("compare fail  to json string")
    );
    let _ = compare_res.write_all(compare_str.as_bytes());
    return (true, compare.diff_amount, compare.diff_fee, compare.diff_amount_per);
}

