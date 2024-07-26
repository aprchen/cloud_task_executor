use lambda_runtime::{LambdaEvent, Error};
use serde_json::Value;
use warp::hyper::body::Bytes;
use warp::Filter;
use crate::executor::Executor;
use std::borrow::Cow;
use warp::Reply;

pub async fn handle_lambda_event(executor: Executor, event: LambdaEvent<Value>) -> Result<Value, Error> {
    let (payload, _ctx) = event.into_parts();
    executor.execute_tasks(payload).await;
    Ok(Value::Null)
}

pub async fn handle_fc_request(executor: Executor, body: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    let body_str: Cow<str> = String::from_utf8_lossy(&body);
    let payload: Value = serde_json::from_str(&body_str).unwrap_or(Value::Null);
    executor.execute_tasks(payload).await;
    Ok(warp::reply::json(&serde_json::json!({"message": "finished"})))
}

pub fn create_fc_route(executor: Executor) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("invoke"))
        .and(warp::body::bytes())
        .and_then(move |body: Bytes| {
            let executor = executor.clone();
            async move {
                handle_fc_request(executor, body).await
            }
        })
}