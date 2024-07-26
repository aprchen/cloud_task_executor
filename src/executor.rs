use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::future::BoxFuture;

pub type Context = Arc<Mutex<HashMap<String, String>>>;
pub type TaskFn = Box<dyn Fn(Context) -> BoxFuture<'static, Result<String, String>> + Send + Sync>;
pub type ContextInitializer = Box<dyn Fn(&Context) + Send + Sync>;
pub type PostExecutor = Box<dyn Fn(&Context, Result<String, String>) + Send + Sync>;

pub struct Task {
    name: String,
    task_fn: TaskFn,
}

impl Task {
    pub fn new<T>(name: &str, task_fn: T) -> Self
        where
            T: Fn(Context) -> BoxFuture<'static, Result<String, String>> + 'static + Send + Sync,
    {
        Self {
            name: name.to_string(),
            task_fn: Box::new(task_fn),
        }
    }

    pub async fn execute(&self, ctx: Context) -> Result<String, String> {
        (self.task_fn)(ctx).await
    }
}

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
            context: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn register_context_initializer<C>(&mut self, initializer: C)
        where
            C: Fn(&Context) + 'static + Send + Sync,
    {
        self.context_initializers.push(Box::new(initializer));
    }

    pub fn register_post_executor<E>(&mut self, executor: E)
        where
            E: Fn(&Context, Result<String, String>) + 'static + Send + Sync,
    {
        self.post_executors.push(Box::new(executor));
    }

    pub async fn run(&self) {
        for initializer in &self.context_initializers {
            initializer(&self.context);
        }

        for task in &self.tasks {
            let result = task.execute(self.context.clone()).await;
            for executor in &self.post_executors {
                executor(&self.context, result.clone());
            }
        }
    }
}