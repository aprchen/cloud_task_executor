use std::collections::HashMap;
use std::fmt::Display;
use std::string::ToString;
use std::sync::{Arc, Mutex};
use futures::future::BoxFuture;
use lambda_runtime::{service_fn, LambdaEvent};
use serde_json::Value;
use structopt::StructOpt;
use crate::cloud_providers::{handle_lambda_event, create_fc_route};
use crate::args::Args;

pub enum Runtime {
    FC,
    Lambda,
    Local,
}

impl Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Runtime::FC => "fc".to_string(),
            Runtime::Lambda => "lambda".to_string(),
            Runtime::Local => "local".to_string(),
        };
        write!(f, "{}", str)
    }
}

pub const KEY_RUNTIME: &str = "task_runtime";

pub type TaskFn = Arc<dyn Fn(Context, Value) -> BoxFuture<'static, Result<String, String>> + Send + Sync>;
pub type Initializer = Arc<dyn Fn(&Context) + Send + Sync>;
pub type AfterAction = Arc<dyn Fn(&Context, Value, Result<String, String>) -> Result<String, String> + Send + Sync>;
pub type BeforeAction = Arc<dyn Fn(&Context, Value) -> Value + Send + Sync>;

#[derive(Default,Clone)]
pub struct Context {
    pub data: Arc<Mutex<HashMap<String, String>>>,
}

impl Context {
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.lock().expect("get lock failed").get(key).cloned()
    }

    pub fn set(&self, key: &str, value: String) {
        self.data.lock().expect("set lock failed").insert(key.to_string(), value);
    }
}


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

#[derive(Clone)]
pub struct Executor {
    task: Option<Task>,
    pub(crate) initializer: Option<Initializer>,
    after_action: Option<AfterAction>,
    before_action: Option<BeforeAction>,
    pub(crate) context: Context,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {
            task: None,
            initializer: None,
            after_action: None,
            before_action: None,
            context: Context::default(),
        }
    }

    pub fn set_task(&mut self, task: Task) {
        self.context.set("task_name", task.name.clone());
        self.task = Some(task);
    }

    pub fn set_initializer<C>(&mut self, initializer: C)
    where
        C: Fn(&Context) + 'static + Send + Sync,
    {
        self.initializer = Some(Arc::new(initializer));
    }

    pub fn set_after_action<E>(&mut self, action: E)
    where
        E: Fn(&Context, Value, Result<String, String>) -> Result<String, String> + 'static + Send + Sync,
    {
        self.after_action = Some(Arc::new(action));
    }

    pub fn set_before_action<M>(&mut self, action: M)
    where
        M: Fn(&Context, Value) -> Value + 'static + Send + Sync,
    {
        self.before_action = Some(Arc::new(action));
    }

    fn handle_args(&self) -> Args {
        Args::from_args()
    }

    pub async fn execute_task(&self, payload: Option<Value>) -> Result<String, String> {
        let mut payload: Value = payload.unwrap_or(Value::Null);

        if let Some(action) = &self.before_action {
            payload = action(&self.context, payload);
        }

        let result = if let Some(task) = &self.task {
            task.execute(self.context.clone(), payload.clone()).await
        } else {
            Err("No task registered".to_string())
        };

        if let Some(action) = &self.after_action {
            action(&self.context, payload.clone(), result)
        } else {
            result
        }
    }

    pub async fn run(self) -> Result<String, String> {
        self.context.set(KEY_RUNTIME, get_runtime().to_string());
        if let Some(initializer) = &self.initializer {
            initializer(&self.context);
        }
        match get_runtime() {
            Runtime::FC => {
                let route = create_fc_route(self);
                warp::serve(route).run(([0, 0, 0, 0], 9000)).await;
                Ok("FC function executed".to_string())
            }
            Runtime::Lambda => {
                let func = service_fn(move |event: LambdaEvent<Value>| {
                    let executor = self.clone();
                    async move {
                        handle_lambda_event(executor, event).await
                    }
                });
                lambda_runtime::run(func).await.expect("Failed to run AWS Lambda function");
                Ok("AWS Lambda function executed".to_string())
            }
            Runtime::Local => {
                let args = self.handle_args();
                let result = self.execute_task(args.payload).await;
                Ok(result.unwrap_or_else(|err| err))
            }
        }
    }
}

pub fn get_runtime() -> Runtime {
    if std::env::var("FC_REGION").is_ok() {
        Runtime::FC
    } else if std::env::var("AWS_REGION").is_ok() {
        Runtime::Lambda
    } else {
        Runtime::Local
    }
}