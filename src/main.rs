use std::env;
use std::time::Duration;
use reqwest::Url;


use parse_logs::{Config, parse_logs_fn};
use parse_logs::init_log;
use parse_logs::RouterApiClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("only need one arg used as config file");
        return Ok(());
    }
    let config_file = (&args[1]).clone();
    let config = Config::from_file(config_file.as_str());
    init_log("info");
    let mut client = RouterApiClient::new(
        Url::parse(config.old_url.as_str()).expect("decode old url fail"),
        Url::parse(config.new_url.as_str()).expect("decode new url fail"),
        Duration::from_secs(15),
    );
    parse_logs_fn(&mut client, config).await?;
    // prase_logs(file_path)
    Ok(())
}
