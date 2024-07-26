use structopt::StructOpt;
use serde_json::Value;

#[derive(StructOpt, Debug)]
#[structopt(name = "cloud_task_executor")]
pub struct Args {
    /// The payload for the task
    #[structopt(short, long, parse(try_from_str = parse_json))]
    pub payload: Option<Value>,
}

fn parse_json(src: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(src)
}