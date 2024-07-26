# CloudTaskExecutor
阿里云fc&amp;&amp;aws lambda 执行器


## 使用方式, 请参考examples目录下的示例代码

- 存在 LAMBDA_TASK_ROOT 则认为是在lambda环境下执行
- 存在 FC_FUNC_CODE_PATH 则认为是在fc环境下执行
- 以上两个环境变量都不存在则认为是在本地环境下执行

```rust
use cloud_task_executor::*;
use serde_json::Value;

#[cte_task(name = "my_task")]
async fn my_task(ctx: Context, payload: Value) -> Result<String, String> {
    let sample_value = ctx.get("sample_key").expect("sample_key not found");
    let payload_str = payload.get("payload_key").and_then(Value::as_str).unwrap_or("default_value");
    println!("Task running with sample value: {}, payload: {}", sample_value, payload_str);
    Ok("Task result".to_string())
}

#[tokio::main]
async fn main(){
    let mut executor = Executor::new();
    executor.set_initializer(|ctx| {
        ctx.set("sample_key", "sample_value".to_string());
    });
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
    // 注册任务
    executor.register_task(my_task());
    executor.run().await.expect("Executor failed to run");
}

```

```shell
# 本地执行
cargo run -- --payload '{}'
```