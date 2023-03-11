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
    let (mut old_file, mut new_file, mut compare_detail_file, mut compare_file) = get_output_files(&config);
    let mut index: u64 = 0;
    let mut pos: u64 = 0;
    let mut neg: u64 = 0;
    let mut diff_pers: Vec<f64> = vec![0.0; 8];
    let mut token_pair_maps = HashMap::new();

    let mut path_size_pass: Vec<i64> = vec![0; 5];
    let mut path_size_count: Vec<i64> = vec![0; 5];
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

        if let Ok((old_res, new_res, cast)) = call_router_servers(client, &log_content).await {
            let (res, diff_amount, _, diff_amount_per, path_size) = save_results(
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
            update_clc_paths(&mut path_size_pass, &mut path_size_count, cast, path_size);
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
    let cast_res = clc_path_to_string(&mut path_size_pass, &mut path_size_count);
    let _ = compare_file.write_all(cast_res.as_bytes());

    Ok(())
}


fn update_clc_paths(path_size_pass: &mut Vec<i64>, path_size_count: &mut Vec<i64>, cast: i64, size: u32) {
    let index;
    if size < 50 {
        index = 0;
    } else if 50 <= size && size < 150 {
        index = 1;
    } else if 150 <= size && size < 300 {
        index = 2;
    } else if 300 <= size && size < 600 {
        index = 3;
    } else {
        index = 4;
    }
    path_size_pass[index] += cast;
    path_size_count[index] += 1;
}

fn clc_path_to_string(path_size_pass: &mut Vec<i64>, path_size_count: &mut Vec<i64>) -> String {
    let sum_count: i64 = path_size_count.iter().sum();
    let sum_pass: i64 = path_size_pass.iter().sum();
    let per_pass = sum_pass as f64 / sum_count as f64;
    let mut pass_res = vec![0.0; 5];
    for i in 0..5 {
        pass_res[i] = path_size_pass[i] as f64 / path_size_count[i] as f64;
    }


    format!(
        "Path size [0, 50)  num: {} per Cast:{} \n\
         Path size [50, 150) num: {} per Cast:{} \n\
         Path size [150, 300) num: {} per Cast:{} \n\
         Path size [300, 600) num: {} per Cast:{} \n\
         Path size [600, ....) num: {} per Cast:{} \nSum per :{}",
        path_size_count[0], pass_res[0],
        path_size_count[1], pass_res[1],
        path_size_count[2], pass_res[2],
        path_size_count[3], pass_res[3],
        path_size_count[4], pass_res[4],
        per_pass
    )
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
        "count:{}, diff <0.01%: {}%,  0.01%~0.1%:{}%, 0.1%~1%:{}%, 1%~2%:{}%, 2%~5%:{}%, 5%~10%:{}%, >10%:{}%\n",
        count,
        diff_pers[0] * 100.0,
        diff_pers[2] * 100.0,
        diff_pers[3] * 100.0,
        diff_pers[4] * 100.0,
        diff_pers[5] * 100.0,
        diff_pers[6] * 100.0,
        diff_pers[7] * 100.0,
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
    compare_res: &mut File) -> (bool, f64, f64, f64, u32) {
    let item_old = old.data.as_ref().unwrap().get(0).unwrap();
    let item_new = new.data.as_ref().unwrap().get(0).unwrap();
    if item_old.amount.is_none() {
        return (false, 0.0, 0.0, 0.0, 0);
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
    return (true, compare.diff_amount, compare.diff_fee, compare.diff_amount_per, compare.paths);
}

