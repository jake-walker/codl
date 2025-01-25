use std::{env, io};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::process::exit;
use codl;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args();

    if args.len() != 2 {
        println!("usage: codl <url>");
        exit(1);
    }

    let instance_url = env::var("INSTANCE_URL").unwrap();
    let auth_token= env::var("AUTH_TOKEN").ok();

    let client = codl::Client::new(instance_url, auth_token)?;

    let res = client.download(&args.last().unwrap()).await?;
    let mut reader = BufReader::new(res.data.as_ref());

    let mut f = File::create(res.filename.clone())?;
    io::copy(&mut reader, &mut f)?;

    println!("saved to {}", res.filename);

    Ok(())
}
