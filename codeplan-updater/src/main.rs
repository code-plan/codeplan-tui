use reqwest::Client;
use std::fs::File;
use std::io::prelude::*;

type Error = Box<dyn std::error::Error>;
type Result<T, E = Error> = std::result::Result<T, E>;

async fn get_tasks() -> Result<()> {
    let mut file = File::create("./cache/task.json")?;
    let client = Client::new();
    let req = client
        .get("http://172.30.152.201:4000/tasks/")
        .header("Accepts", "application/json");
    let res = req.send().await?;
    let body = res.bytes().await?;
    let v = body.to_vec();
    let s = String::from_utf8_lossy(&v);
    let string_body = String::from(s);
    file.write_all(string_body.as_ref())?;
    Ok(())
}

async fn get_comments() -> Result<()> {
    let mut file = File::create("./cache/comment.json")?;
    let client = Client::new();
    let req = client
        .get("http://172.30.152.201:4000/tasks/comments/")
        .header("Accepts", "application/json");
    let res = req.send().await?;
    let body = res.bytes().await?;
    let v = body.to_vec();
    let s = String::from_utf8_lossy(&v);
    let string_body = String::from(s);
    file.write_all(string_body.as_ref())?;
    Ok(())
}

async fn get_projects() -> Result<()> {
    let mut file = File::create("./cache/project.json")?;
    let client = Client::new();
    let req = client
        .get("http://172.30.152.201:4000/projects/")
        .header("Accepts", "application/json");
    let res = req.send().await?;
    let body = res.bytes().await?;
    let v = body.to_vec();
    let s = String::from_utf8_lossy(&v);
    let string_body = String::from(s);
    file.write_all(string_body.as_ref())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    get_tasks().await?;
    get_comments().await?;
    get_projects().await?;
    Ok(())
}