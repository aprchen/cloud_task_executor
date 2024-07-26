use crate::executor::Executor;
use lambda_runtime::{Error, LambdaEvent};
use serde_json::{json, Value};
use std::borrow::Cow;
use warp::hyper::body::Bytes;
use warp::{Filter, Rejection};
use warp::Reply;

pub async fn handle_lambda_event(
    executor: Executor,
    event: LambdaEvent<Value>,
) -> Result<Value, Error> {
    let (payload, _ctx) = event.into_parts();
    let result = executor.execute_task(Some(payload)).await;
    match result {
        Ok(data) => Ok(json!({ "status": "success", "data": data })),
        Err(err) => Ok(json!({ "status": "error", "data": err })),
    }
}

pub async fn handle_fc_request(
    executor: Executor,
    body: Bytes,
) -> Result<impl Reply, Rejection> {
    let body_str: Cow<str> = String::from_utf8_lossy(&body);
    let payload: Value = serde_json::from_str(&body_str).unwrap_or(Value::Null);
    let result = executor.execute_task(Some(payload)).await;
    match result {
        Ok(data) => Ok(warp::reply::json(&json!({ "status": "success", "data": data }))),
        Err(err) => Ok(warp::reply::json(&json!({ "status": "error", "data": err }))),
    }
}

pub fn create_fc_route(
    executor: Executor,
) -> impl Filter<Extract=impl Reply, Error=warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("invoke"))
        .and(warp::body::bytes())
        .and_then(move |body: Bytes| {
            let executor = executor.clone();
            async move { handle_fc_request(executor, body).await }
        })
}
