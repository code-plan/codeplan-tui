use std::env;
use std::io::{Read, Write};
use std::io::prelude::*;

use reqwest::Client;

type Error = Box<dyn std::error::Error>;
type Result<T, E = Error> = std::result::Result<T, E>;

async fn complete_task(task_id: &str) -> Result<()> {
    let mut url = "http://172.30.152.201:4000/tasks/".to_owned();
    url.push_str(task_id);
    url.push_str("/");
    url.push_str("complete");
    let client = Client::new();
    let req = client
        // or use .post, etc.
        .post(&url)
        .header("Accepts", "application/json");
    let res = req.send().await?;
    let body = res.bytes().await?;
    let v = body.to_vec();
    let s = String::from_utf8_lossy(&v);
    let body_json = String::from(s);
    Ok(())
}

async fn delete_task(task_id: &str) -> Result<()> {
    let mut url = "http://172.30.152.201:4000/tasks/".to_owned();
    url.push_str(task_id);
    let client = Client::new();
    let req = client
        // or use .post, etc.
        .delete(&url)
        .header("Accepts", "application/json");
    let res = req.send().await?;
    let body = res.bytes().await?;
    let v = body.to_vec();
    let s = String::from_utf8_lossy(&v);
    let body_json = String::from(s);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.contains(&String::from("-complete")) {
        if args.len() > 2 {
            let task_index: usize = args.iter().position(|r| r == "-complete").unwrap() + 1;
            complete_task(&args[task_index]).await?;
        } else { println!("Missing or incorrect arguments.") }
    } else if args.contains(&String::from("-delete")) {
        if args.len() > 2 {
            let task_index: usize = args.iter().position(|r| r == "-delete").unwrap() + 1;
            delete_task(&args[task_index]).await?;
        } else { println!("Missing or incorrect arguments.") }
    }
    Ok(())
}