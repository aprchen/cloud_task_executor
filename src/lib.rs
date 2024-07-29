//! # Cloud Task Executor
//!
//! The Cloud Task Executor is a versatile and powerful framework designed to simplify the execution of tasks in cloud environments such as AWS Lambda and Alibaba Cloud Function Compute (FC). It provides a unified interface for registering and executing tasks, managing execution contexts, and handling pre- and post-execution actions. This flexibility allows developers to focus on the core logic of their tasks without worrying about the underlying cloud infrastructure.
//!
//! ## Features
//!
//! - **Unified Task Registration**: Register tasks using a simple and consistent interface, making it easy to manage tasks across different cloud environments.
//! - **Context Management**: Efficiently manage execution contexts with thread-safe access to shared data.
//! - **Pre- and Post-Execution Actions**: Define actions to be executed before and after the main task execution, enabling additional processing and context modification.
//! - **Cloud-Agnostic Execution**: Seamlessly execute tasks on AWS Lambda, Alibaba Cloud FC, or local development environments without changing the core task logic.
//! - **Automatic Payload Handling**: Simplify task payload handling with built-in JSON parsing and context initialization.
//!
//! ## Example
//!
//! ```rust
//! use cloud_task_executor::*;
//! use serde_json::{json,Value};
//!
//! #[cte_task(name = "my_task")]
//! async fn my_task(ctx: Context, payload: Value) -> Result<String, String> {
//!     let sample_value = ctx.get("sample_key").expect("sample_key not found");
//!     let payload_str = payload.get("payload_key").and_then(Value::as_str).unwrap_or("default_value");
//!     let runtime = ctx.get(KEY_RUNTIME).unwrap();
//!     println!("Task running with sample value: {}, payload: {}, runtime {}", sample_value, payload_str, runtime);
//!     Ok("Task result".to_string())
//! }
//!
//! #[tokio::main]
//! async fn main(){
//!     let mut executor = Executor::new();
//!     executor.set_initializer(|ctx| {
//!         ctx.set("sample_key", "sample_value".to_string());
//!     });
//!     executor.set_after_action(|_ctx, payload, result| {
//!         println!("Task executed with payload: {:?}, result: {:?}", payload, result);
//!         result.map(|res| format!("{} - after action", res))
//!     });
//!     executor.set_before_action(|ctx, payload| {
//!        ctx.set("modified_key", "test".to_string());
//!         json!({"test":1})
//!     });
//!     // 注册任务
//!     executor.set_task(my_task());
//!     executor.run().await.expect("Executor failed to run");
//! }
//! ```
pub mod executor;
pub mod cloud_providers;
pub mod args;

pub use executor::{Executor, Task, Context, Runtime, KEY_RUNTIME};
pub use cloud_providers::{handle_lambda_event, create_fc_route};
pub use args::Args;

// Re-export the macro
pub use cte_basic_macros::cte_task;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn initialize_executor() -> Executor {
        let mut executor = Executor::new();

        executor.set_initializer(|ctx| {
            ctx.set("sample_key", "sample_value".to_string());
        });

        // 注册任务
        executor.set_task(my_task());

        executor.set_after_action(|_ctx, payload, result| {
            println!("Task executed with payload: {:?}, result: {:?}", payload, result);
            result.map(|res| format!("{} - after action", res))
        });

        executor.set_before_action(|ctx, payload| {
            if let Some(value) = payload.get("modify_key").and_then(Value::as_str) {
                ctx.set("modified_key", value.to_string());
            }
            ctx.set("my_task", "Task result".to_string());
            let mut new_payload = json!({"test": 1});
            if let Value::Object(map) = payload {
                if let Value::Object(new_map) = &mut new_payload {
                    for (k, v) in map {
                        new_map.insert(k.clone(), v.clone());
                    }
                }
            }
            new_payload
        });

        executor
    }

    // 使用宏定义任务
    #[cte_task(name = "my_task")]
    async fn my_task(ctx: Context, payload: Value) -> Result<String, String> {
        let sample_value = ctx.get("sample_key").expect("sample_key not found");
        let payload_str = payload.get("payload_key").and_then(Value::as_str).unwrap_or("default_value");
        println!("Task running with sample value: {}, payload: {}", sample_value, payload_str);
        Ok("Task result".to_string())
    }

    #[tokio::test]
    async fn test_my_task() {
        let mut executor = initialize_executor();

        // 初始化上下文
        if let Some(initializer) = &executor.initializer {
            initializer(&executor.context);
        }

        let payload = Some(json!({"payload_key": "test_value", "modify_key": "modified_value"}));
        let result = executor.execute_task(payload.clone()).await;

        {
            let context = &executor.context;
            let result_value = context.get("my_task").expect("Result not found");
            assert_eq!(result_value, "Task result");
            assert_eq!(result, Ok("Task result - after action".to_string()));

            let modified_value = context.get("modified_key").expect("Modified key not found");
            assert_eq!(modified_value, "modified_value");
        }

        // 重新初始化 executor 并测试没有提供 payload 的情况
        executor = initialize_executor();

        // 初始化上下文
        if let Some(initializer) = &executor.initializer {
            initializer(&executor.context);
        }

        let result = executor.execute_task(None).await;
        {
            let context = &executor.context;
            let result_value = context.get("my_task").expect("Result not found");
            assert_eq!(result_value, "Task result");
            assert_eq!(result, Ok("Task result - after action".to_string()));
        }
    }
}