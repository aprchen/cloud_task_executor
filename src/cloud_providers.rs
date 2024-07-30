use crate::executor::Executor;
use lambda_runtime::{Error, LambdaEvent};
use serde_json::{json, Value};
use std::borrow::Cow;
use log::debug;
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
    let mut payload: Value = serde_json::from_str(&body_str).unwrap_or(Value::Null);
    debug!("Received FC request: {:?}", payload);
    // 提取 payload 内的值并合并到顶层，并移除原始 payload 键，抹平sdk invoke和scheduler invoke 的差异
    if let Some(payload_str) = payload.get("payload").and_then(|v| v.as_str()) {
        if let Ok(parsed_payload) = serde_json::from_str::<Value>(payload_str) {
            // 提取 payload 内的值并合并到顶层，并移除原始 payload 键
            if let Some(inner_map) = parsed_payload.as_object() {
                for (key, value) in inner_map {
                    payload[key] = value.clone();
                }
            }
            payload.as_object_mut().unwrap().remove("payload");
        }
    }
    debug!("Transformed FC request: {:?}", payload);
    let result = executor.execute_task(Some(payload)).await;
    match result {
        Ok(data) => Ok(warp::reply::json(&json!({ "status": "success", "data": data }))),
        Err(err) => Ok(warp::reply::json(&json!({ "status": "error", "data": err }))),
    }
}

pub fn create_fc_route(
    executor: Executor,
) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::post()
        .and(warp::path("invoke"))
        .and(warp::body::bytes())
        .and_then(move |body: Bytes| {
            let executor = executor.clone();
            async move { handle_fc_request(executor, body).await }
        })
}