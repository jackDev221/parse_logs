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
    let (mut compare_detail_file, mut compare_file, mut restore_file, mut token_pair_file) = get_output_files(&config);
    let mut index: u64 = 0;
    let mut diff_amount_pers = init_diff_pers();
    let mut diff_fee_pers = init_diff_pers();
    let mut diff_impact_pers = init_diff_pers();
    let mut diff_inusd_pers = init_diff_pers();
    let mut diff_ount_pers = init_diff_pers();

    let mut path_cast: Vec<i64> = vec![0; 5];
    let mut path_count: Vec<i64> = vec![0; 5];

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
        let is_contain = token_pair_maps.contains_key(&key);
        if  is_contain {
            if config.rm_duplicate {
                continue;
            }
        } else {
            token_pair_maps.insert(key.clone(), key);
            if config.restore_input {
                let _ = restore_file.write_all(format!("{}\n", line_content).as_bytes());
            }
        }

        if let Ok((old_res, new_res, cast)) = call_router_servers(client, &log_content).await {
            update_clc_paths(&mut path_cast, &mut path_count, cast, &new_res, &mut token_pair_file, is_contain);


            let (_, res) = compare_results(
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

    let path_info = clc_path_to_string(&path_cast, &path_count);
    let _ = compare_file.write_all(
        format!(
            "Performs:\n{}\n",
            path_info
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


fn update_clc_paths(path_cast: &mut Vec<i64>, path_count: &mut Vec<i64>, cast: i64, router_result: &RouterResult, token_pair_file: &mut File, is_exist: bool) {
    let path = router_result.data.as_ref().unwrap().get(0).unwrap();
    if path.amount.is_none() {
        return;
    }
    let size = path.paths;
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
    path_cast[index] += cast;
    path_count[index] += 1;
    if size > 50 && !is_exist {
        let roads = path.road_for_name.as_ref().unwrap();
        let addrs = path.road_for_addr.as_ref().unwrap();
        let _ = token_pair_file.write_all(
            format!(
                "From:{}, To:{}, FromAddr:{}, ToAddr:{} , size:{}\n",
                roads[0],
                roads[roads.len() - 1],
                addrs[0],
                addrs[roads.len() - 1],
                size
            ).as_bytes()
        );
    }
}

fn clc_path_to_string(path_cast: &Vec<i64>, path_count: &Vec<i64>) -> String {
    let sum_count: i64 = path_count.iter().sum();
    let sum_pass: i64 = path_cast.iter().sum();
    let per_pass = sum_pass as f64 / sum_count as f64;
    let mut pass_res = vec![0.0; 5];
    for i in 0..5 {
        pass_res[i] = path_cast[i] as f64 / path_count[i] as f64;
    }

    format!(
        "Path size [0, 50)  num: {} per Cast:{} \n\
         Path size [50, 150) num: {} per Cast:{} \n\
         Path size [150, 300) num: {} per Cast:{} \n\
         Path size [300, 600) num: {} per Cast:{} \n\
         Path size [600, ....) num: {} per Cast:{} \nSum per :{}",
        path_count[0], pass_res[0],
        path_count[1], pass_res[1],
        path_count[2], pass_res[2],
        path_count[3], pass_res[3],
        path_count[4], pass_res[4],
        per_pass
    )
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

fn get_output_files(config: &Config) -> (File, File, File, File) {
    let compare_detail = OpenOptions::new().create(true).write(true).append(true).open(config.compare_res_detail_path.as_str()).unwrap();
    let compare = OpenOptions::new().create(true).write(true).append(true).open(config.compare_res_path.as_str()).unwrap();
    let restore_input = OpenOptions::new().create(true).write(true).append(true).open(config.restore_input_path.as_str()).unwrap();
    let token_pair_file = OpenOptions::new().create(true).write(true).append(true).open(config.token_pair_of_large_paths.as_str()).unwrap();
    (compare_detail, compare, restore_input, token_pair_file)
}

fn compare_results(
    index: u64,
    log_origin: String,
    old: &RouterResult,
    new: &RouterResult,
    compare_res: &mut File,
) -> (f64, Vec<CompareResult>) {
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

