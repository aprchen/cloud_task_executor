mod cloud_providers;
mod executor;

use basic_macros::cte_task;
use executor::{Context, Executor, WrapperFn};
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

struct DbConnection {
    connection_string: String,
}

#[tokio::main]
async fn main() {
    let mut executor = Executor::new();

    executor.register_context_initializer(|ctx| {
        let db_conn = DbConnection {
            connection_string: "DB Connection String".to_string(),
        };
        ctx.lock().unwrap().initializers.insert(
            "db_connection".to_string(),
            Arc::new(db_conn) as Arc<dyn Any + Send + Sync>,
        );
    });

    // 定义一个简单的包装函数，修改传入的参数
    let param_wrapper: WrapperFn = Arc::new(|task_fn| {
        Arc::new(move |ctx, mut payload| {
            payload["payload_key"] = Value::String("modified_value".to_string());
            task_fn(ctx, payload)
        })
    });

    // 定义一个简单的包装函数，修改返回结果
    let result_wrapper: WrapperFn = Arc::new(|task_fn| {
        Arc::new(move |ctx, payload| {
            let task_fn = task_fn.clone();
            Box::pin(async move {
                let result = task_fn(ctx, payload).await?;
                Ok(format!("{} - modified", result))
            })
        })
    });

    // 注册任务并应用包装函数
    executor.register_task(
        my_task()
            .with_wrapper(&param_wrapper)
            .with_wrapper(&result_wrapper),
    );
    executor.register_task(
        test()
            .with_wrapper(&param_wrapper)
            .with_wrapper(&result_wrapper),
    );

    executor.register_post_executor(|ctx| {
        let context = ctx.lock().unwrap();
        for (task_name, result) in &context.results {
            let result = result.downcast_ref::<Result<String, String>>();
            match result {
                Some(res) => println!("Task {} completed with result: {:?}", task_name, res),
                None => println!("Task {} result is of an unexpected type", task_name),
            }
        }
    });

    executor.run().await;
}

#[cte_task(name = "test")]
async fn test(_ctx: Context, _payload: Value) -> Result<String, String> {
    Ok("test".to_string())
}

// 使用宏定义任务
#[cte_task(name = "my_task")]
async fn my_task(ctx: Context, payload: Value) -> Result<String, String> {
    let binding = ctx.lock().unwrap();
    let db_conn = binding
        .initializers
        .get("db_connection")
        .and_then(|conn| conn.downcast_ref::<DbConnection>())
        .expect("DB connection not found");
    let payload_str = payload
        .get("payload_key")
        .and_then(Value::as_str)
        .unwrap_or("default_value");
    println!(
        "Task running with DB connection: {}, payload: {}",
        db_conn.connection_string, payload_str
    );
    Ok("Task result".to_string())
}
