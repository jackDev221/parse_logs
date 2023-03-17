use std::collections::HashMap;
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
    let (mut compare_detail_file, mut compare_file) = get_output_files(&config);
    let mut index: u64 = 0;
    let mut diff_amount_pers = init_diff_pers();
    let mut diff_fee_pers = init_diff_pers();
    let mut diff_impact_pers = init_diff_pers();
    let mut diff_inusd_pers = init_diff_pers();
    let mut diff_ount_pers = init_diff_pers();
    let mut count_path = vec![0.0; 2];

    let mut token_pair_maps = HashMap::new();

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

        let key = format!("{}_{}", log_content.from_token, log_content.to_token);
        if token_pair_maps.contains_key(&key) {
            continue;
        } else {
            token_pair_maps.insert(key.clone(), key);
        }

        if let Ok((old_res, new_res, _)) = call_router_servers(client, &log_content).await {
            let (count, res) = compare_results(
                index,
                serde_json::to_string(&log_content).unwrap(),
                &old_res,
                &new_res,
                &mut compare_detail_file,
            );

            for i in 0..res.len() {
                let com_res = &res[i];
                calc_compare_res(&mut diff_amount_pers, com_res.diff_amount_per);
                calc_compare_res(&mut diff_fee_pers, com_res.diff_fee_per);
                calc_compare_res(&mut diff_impact_pers, com_res.diff_impact_per);
                calc_compare_res(&mut diff_inusd_pers, com_res.diff_inusd_per);
                calc_compare_res(&mut diff_ount_pers, com_res.diff_outusd_per);
            }
            count_path[1] += res.len() as f64;
            count_path[0] += count - res.len() as f64;

            index += 1;
        } else {
            warn!("Fail to get response for {}", line_content);
        }
    }
    write_compare_result("Amount".to_owned(), &mut diff_amount_pers, &mut compare_file);
    write_compare_result("Fee".to_owned(), &mut diff_fee_pers, &mut compare_file);
    write_compare_result("impact".to_owned(), &mut diff_impact_pers, &mut compare_file);
    write_compare_result("Inusd".to_owned(), &mut diff_inusd_pers, &mut compare_file);
    write_compare_result("Outusd".to_owned(), &mut diff_ount_pers, &mut compare_file);

    let count: f64 = count_path.iter().sum();
    for i in 0..count_path.len() {
        count_path[i] /= count;
    }

    let _ = compare_file.write_all(
        format!(
            "sum:{}, diff:{}% same:{}%\n",
            count,
            count_path[0] * 100.0,
            count_path[1] * 100.0
        ).as_bytes()
    );
    Ok(())
}

fn write_compare_result(tag: String, pers: &mut Vec<f64>, compare_res: &mut File) {
    let _ = compare_res.write_all(format!("{}: diff\n", tag).as_bytes());
    let res = compare_res_to_string(pers);
    let _ = compare_res.write_all(
        format!(
            " {}\n",
            res
        ).as_bytes());
}

pub fn write_paths(path_diff: &mut Vec<Vec<f64>>, compare_res: &mut File) {
    let _ = compare_res.write_all(format!("Pool and paths: diff\n").as_bytes());
    for i in 0..3 {
        let path = &mut path_diff[i];
        let count: f64 = path.iter().sum();
        for i in 0..path.len() {
            path[i] /= count;
        }
        let _ = compare_res.write_all(
            format!(
                "path:{} sum:{}, diff:{}% same:{}%\n",
                i,
                count,
                path[0] * 100.0,
                path[1] * 100.0
            ).as_bytes()
        );
    }
}

fn init_diff_pers() -> Vec<f64> {
    vec![0.0; 7]
}


fn calc_compare_res(diff_pers: &mut Vec<f64>, diff_per: f64) {
    let index;
    if diff_per < 0.0001 {
        index = 0;
    } else if 0.0001 <= diff_per && diff_per < 0.001 {
        index = 1;
    } else if 0.001 <= diff_per && diff_per < 0.01 {
        index = 2;
    } else if 0.01 <= diff_per && diff_per < 0.02 {
        index = 3;
    } else if 0.02 <= diff_per && diff_per < 0.05 {
        index = 4;
    } else if 0.05 <= diff_per && diff_per < 0.1 {
        index = 5;
    } else {
        index = 6
    }
    diff_pers[index] += 1.0;
}

fn compare_res_to_string(diff_pers: &mut Vec<f64>) -> String {
    let count: f64 = diff_pers.iter().sum();
    for i in 0..diff_pers.len() {
        diff_pers[i] /= count;
    }
    format!(
        "count:{}, diff <0.01%: {}%,  0.01%~0.1%:{}%, 0.1%~1%:{}%, 1%~2%:{}%, 2%~5%:{}%, 5%~10%:{}%, >10%:{}%",
        count,
        diff_pers[0] * 100.0,
        diff_pers[1] * 100.0,
        diff_pers[2] * 100.0,
        diff_pers[3] * 100.0,
        diff_pers[4] * 100.0,
        diff_pers[5] * 100.0,
        diff_pers[6] * 100.0,
    )
}

async fn call_router_servers(
    client: &mut RouterApiClient,
    log_content: &LogContent,
) -> anyhow::Result<(RouterResult, RouterResult, i64)> {
    let old_res = client.call_old_router(log_content).await?;
    let t0 = chrono::Utc::now().timestamp_millis();
    let new_res = client.call_new_router(log_content).await?;
    let t1 = chrono::Utc::now().timestamp_millis();
    Ok((old_res, new_res, t1 - t0))
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

fn get_output_files(config: &Config) -> (File, File) {
    let compare_detail = OpenOptions::new().create(true).write(true).append(true).open(config.compare_res_detail_path.as_str()).unwrap();
    let compare = OpenOptions::new().create(true).write(true).append(true).open(config.compare_res_path.as_str()).unwrap();
    (compare_detail, compare)
}

fn compare_results(
    index: u64,
    log_origin: String,
    old: &RouterResult,
    new: &RouterResult,
    compare_res: &mut File) -> (f64, Vec<CompareResult>) {
    let old_paths = old.data.clone().unwrap();
    let new_paths = new.data.clone().unwrap();
    let size = old_paths.len();
    let mut res: Vec<CompareResult> = vec![];
    let mut count = 0.0;
    for i in 0..size {
        let old_path = old_paths.get(i).unwrap();
        let size_new = new_paths.len();
        if old_path.amount.is_none() {
            continue;
        }
        count += 1.0;
        for j in 0..size_new {
            let new_path = new_paths.get(j).unwrap();
            let compare_op = CompareResult::gen_from_paths(old_path, new_path);
            if compare_op.is_none() {
                continue;
            }

            let compare = compare_op.unwrap();
            if compare.pool_eq && compare.road_addr_eq {
                if compare.diff_amount_per > 0.01 {
                    let _ = compare_res.write_all(format!("origin log: {}, differ:{} \n", log_origin, compare.diff_amount_per).as_bytes());
                    let _ = compare_res.write_all(format!(
                        "index:{} path_index:{}\nold:{}\nnew:{}\n",
                        index,
                        i,
                        serde_json::to_string(old_path).unwrap(),
                        serde_json::to_string(new_path).unwrap()
                    ).as_bytes());
                }
                res.push(compare);
            }
        }
    }
    (count, res)
}

