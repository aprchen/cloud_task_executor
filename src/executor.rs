use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use futures::future::BoxFuture;
use lambda_runtime::{service_fn, LambdaEvent};
use serde_json::Value;
use structopt::StructOpt;
use crate::cloud_providers::{handle_lambda_event, create_fc_route};
use crate::args::Args;

#[derive(Default)]
pub struct ContextData {
    pub data: Mutex<HashMap<String, String>>,
}

pub type Context = Arc<ContextData>;
pub type TaskFn = Arc<dyn Fn(Context, Value) -> BoxFuture<'static, Result<String, String>> + Send + Sync>;
pub type ContextInitializer = Arc<dyn Fn(&Context) + Send + Sync>;
pub type AfterAction = Arc<dyn Fn(&Context, Value, Result<String, String>) -> Result<String, String> + Send + Sync>;
pub type PreAction = Arc<dyn Fn(&Context, &Value) + Send + Sync>;

#[derive(Clone)]
pub struct Task {
    name: String,
    task_fn: TaskFn,
}

impl Task {
    pub fn new<T>(name: &str, task_fn: T) -> Self
        where
            T: Fn(Context, Value) -> BoxFuture<'static, Result<String, String>> + 'static + Send + Sync,
    {
        Self {
            name: name.to_string(),
            task_fn: Arc::new(task_fn),
        }
    }

    pub async fn execute(&self, ctx: Context, payload: Value) -> Result<String, String> {
        (self.task_fn)(ctx, payload).await
    }
}

impl ContextData {
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.lock().expect("get lock failed").get(key).cloned()
    }

    pub fn set(&self, key: &str, value: String) {
        self.data.lock().expect("set lock failed").insert(key.to_string(), value);
    }
}

#[derive(Clone)]
pub struct Executor {
    task: Option<Task>,
    pub(crate) context_initializer: Option<ContextInitializer>,
    after_action: Option<AfterAction>,
    pre_action: Option<PreAction>,
    pub(crate) context: Context,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            task: None,
            context_initializer: None,
            after_action: None,
            pre_action: None,
            context: Arc::new(ContextData::default()),
        }
    }

    pub fn register_task(&mut self, task: Task) {
        self.context.set("task_name", task.name.clone());
        self.task = Some(task);
    }

    pub fn set_context_initializer<C>(&mut self, initializer: C)
        where
            C: Fn(&Context) + 'static + Send + Sync,
    {
        self.context_initializer = Some(Arc::new(initializer));
    }

    pub fn set_after_action<E>(&mut self, action: E)
        where
            E: Fn(&Context, Value, Result<String, String>) -> Result<String, String> + 'static + Send + Sync,
    {
        self.after_action = Some(Arc::new(action));
    }

    pub fn set_pre_action<M>(&mut self, action: M)
        where
            M: Fn(&Context, &Value) + 'static + Send + Sync,
    {
        self.pre_action = Some(Arc::new(action));
    }

    fn handle_args(&self) -> Args {
        Args::from_args()
    }

    pub async fn execute_task(&self, payload: Option<Value>) -> Result<String, String> {
        let payload = payload.unwrap_or_else(|| Value::Null);

        // 调用注册的修改 context 的闭包
        if let Some(action) = &self.pre_action {
            action(&self.context, &payload);
        }

        let result = if let Some(task) = &self.task {
            task.execute(self.context.clone(), payload.clone()).await
        } else {
            Err("No task registered".to_string())
        };

        // 在任务执行完毕后调用后处理函数
        let result = if let Some(action) = &self.after_action {
            action(&self.context, payload.clone(), result)
        } else {
            result
        };

        result
    }

    pub async fn run(self) -> Result<String, String> {
        if let Some(initializer) = &self.context_initializer {
            initializer(&self.context);
        }

        if std::env::var("LAMBDA_TASK_ROOT").is_ok() {
            let func = service_fn(move |event: LambdaEvent<Value>| {
                let executor = self.clone();
                async move {
                    handle_lambda_event(executor, event).await
                }
            });
            lambda_runtime::run(func).await.expect("Failed to run AWS Lambda function");
            Ok("AWS Lambda function executed".to_string())
        } else if std::env::var("FC_FUNC_CODE_PATH").is_ok() {
            let route = create_fc_route(self);
            warp::serve(route).run(([0, 0, 0, 0], 9000)).await;
            Ok("FC function executed".to_string())
        } else {
            // 本地开发环境
            println!("Running in local development environment");
            let args = self.handle_args();
            let result = self.execute_task(args.payload).await;
            Ok(result.unwrap_or_else(|err| err))
        }
    }
}