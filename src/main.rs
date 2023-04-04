use std::env;
use parse_logs::{Config, count_token_pairs};
use parse_logs::init_log;


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
    count_token_pairs(config).await?;
    Ok(())
}

#[tokio::test]
async fn test_client() {
    use parse_logs::LogContent;
    use parse_logs::RouterApiClient;
    use reqwest::Url;
    use std::time::Duration;
    let mut client = RouterApiClient::new(
        Url::parse("http://127.0.0.1:8080/routingInV2").expect("decode old url fail"),
        Url::parse("http://127.0.0.1:8080/routingInV2").expect("decode new url fail"),
        "true".to_owned(),
        Duration::from_secs(15),
    );
    let log_client = LogContent {
        from_token: "USDT".to_string(),
        to_token: "TUSD".to_string(),
        from_token_addr: "TXYZopYRdj2D9XRtbG411XZZ3kM5VkAeBf".to_string(),
        to_token_addr: "TRz7J6dD2QWxBoumfYt4b3FaiRG23pXfop".to_string(),
        in_amount: "20000000".to_string(),
        from_decimal: 6,
        to_decimal: 18,
    };
    let response = client.call_new_router(&log_client).await.expect("Fail to get response");
    println!("{:?}", response);
}