pub mod executor;
pub mod cloud_providers;
pub mod args;

pub use executor::{Executor, Task, Context};
pub use cloud_providers::{handle_lambda_event, create_fc_route};
pub use args::Args;

// Re-export the macro
pub use basic_macros::cte_task;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn initialize_executor() -> Executor {
        let mut executor = Executor::new();

        executor.set_initializer(|ctx| {
            ctx.set("sample_key", "sample_value".to_string());
        });

        // 注册任务
        executor.register_task(my_task());

        executor.set_after_action(|_ctx, payload, result| {
            println!("Task executed with payload: {:?}, result: {:?}", payload, result);
            result.map(|res| format!("{} - after action", res))
        });

        executor.set_pre_action(|ctx, payload| {
            if let Some(value) = payload.get("modify_key").and_then(Value::as_str) {
                ctx.set("modified_key", value.to_string());
            }
            ctx.set("my_task","Task result".to_string())
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
        if let Some(initializer) = &executor.context_initializer {
            initializer(&executor.context);
        }

        let payload = Some(serde_json::json!({"payload_key": "test_value", "modify_key": "modified_value"}));
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
        if let Some(initializer) = &executor.context_initializer {
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