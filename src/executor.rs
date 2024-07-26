use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use futures::future::BoxFuture;
use std::any::Any;
use lambda_runtime::{service_fn, LambdaEvent};
use serde_json::Value;
use crate::cloud_providers::{handle_lambda_event, create_fc_route};

#[derive(Default)]
pub struct ContextData {
    pub initializers: HashMap<String, Arc<dyn Any + Send + Sync>>,
    pub results: HashMap<String, Arc<dyn Any + Send + Sync>>,
}

pub type Context = Arc<Mutex<ContextData>>;
pub type TaskFn = Arc<dyn Fn(Context, Value) -> BoxFuture<'static, Result<String, String>> + Send + Sync>;
pub type ContextInitializer = Arc<dyn Fn(&Context) + Send + Sync>;
pub type PostExecutor = Arc<dyn Fn(&Context) + Send + Sync>;
pub type WrapperFn = Arc<dyn Fn(TaskFn) -> TaskFn + Send + Sync>;

#[derive(Clone)]
pub struct Task {
    name: String,
    task_fn: TaskFn,
    wrappers: Vec<WrapperFn>,
}

impl Task {
    pub fn new<T>(name: &str, task_fn: T) -> Self
        where
            T: Fn(Context, Value) -> BoxFuture<'static, Result<String, String>> + 'static + Send + Sync,
    {
        Self {
            name: name.to_string(),
            task_fn: Arc::new(task_fn),
            wrappers: Vec::new(),
        }
    }

    pub fn with_wrapper(mut self, wrapper: &WrapperFn) -> Self {
        self.wrappers.push(wrapper.clone());
        self
    }

    pub async fn execute(&self, ctx: Context, payload: Value) -> Result<String, String> {
        let mut wrapped_fn = self.task_fn.clone();
        for wrapper in &self.wrappers {
            wrapped_fn = wrapper(wrapped_fn);
        }
        wrapped_fn(ctx, payload).await
    }
}

#[derive(Clone)]
pub struct Executor {
    tasks: Vec<Task>,
    context_initializers: Vec<ContextInitializer>,
    post_executors: Vec<PostExecutor>,
    context: Context,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            context_initializers: Vec::new(),
            post_executors: Vec::new(),
            context: Arc::new(Mutex::new(ContextData::default())),
        }
    }

    pub fn register_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn register_context_initializer<C>(&mut self, initializer: C)
        where
            C: Fn(&Context) + 'static + Send + Sync,
    {
        self.context_initializers.push(Arc::new(initializer));
    }

    pub fn register_post_executor<E>(&mut self, executor: E)
        where
            E: Fn(&Context) + 'static + Send + Sync,
    {
        self.post_executors.push(Arc::new(executor));
    }

    pub async fn execute_tasks(&self, payload: Value) {
        for task in &self.tasks {
            let result = task.execute(self.context.clone(), payload.clone()).await;
            let mut context = self.context.lock().unwrap();
            context.results.insert(task.name.clone(), Arc::new(result) as Arc<dyn Any + Send + Sync>);
        }

        // 在所有任务执行完毕后调用后处理函数
        for executor in &self.post_executors {
            executor(&self.context);
        }
    }

    pub async fn run(self) {
        for initializer in &self.context_initializers {
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
        } else if std::env::var("FC_FUNC_CODE_PATH").is_ok() {
            let route = create_fc_route(self);
            warp::serve(route).run(([0, 0, 0, 0], 9000)).await;
        } else {
            // 本地开发环境
            println!("Running in local development environment");
            let payload = serde_json::json!({});
            self.execute_tasks(payload).await;
        }
    }
}