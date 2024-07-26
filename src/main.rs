mod executor;

use executor::Executor;
use crate::executor::Task;

#[tokio::main]
async fn main() {
    let mut executor = Executor::new();

    executor.register_context_initializer(|ctx| {
        // 注册一些实例，比如数据库连接
        ctx.lock().unwrap().insert("db_connection".to_string(), "DB Connection String".to_string());
    });

    executor.register_task(Task::new("task1", |ctx| {
        Box::pin(async move {
            // 使用 context 获取数据库连接
            let db_conn = ctx.lock().unwrap().get("db_connection").cloned().unwrap_or_default();
            println!("Task 1 is running with DB connection: {}", db_conn);
            Ok("Task 1 result".to_string())
        })
    }));

    executor.register_task(Task::new("task2", |ctx| {
        Box::pin(async move {
            // 使用 context 获取数据库连接
            let db_conn = ctx.lock().unwrap().get("db_connection").cloned().unwrap_or_default();
            println!("Task 2 is running with DB connection: {}", db_conn);
            Ok("Task 2 result".to_string())
        })
    }));

    executor.register_post_executor(|ctx, result| {
        // 处理任务结果，比如发送通知
        if let Ok(res) = result {
            println!("Task completed with result: {:?}", res);
        }
    });

    executor.run().await;
}