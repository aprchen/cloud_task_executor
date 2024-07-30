# Project Description: Cloud Task Executor

## Overview
The Cloud Task Executor is a versatile and powerful framework designed to simplify the execution of tasks in cloud environments such as AWS Lambda and Alibaba Cloud Function Compute (FC). It provides a unified interface for registering and executing tasks, managing execution contexts, and handling pre- and post-execution actions. This flexibility allows developers to focus on the core logic of their tasks without worrying about the underlying cloud infrastructure.
Upper application link [elf_rust](https://github.com/aprchen/elf_rust)

## Key Features

- Unified Task Registration: Register tasks using a simple and consistent interface, making it easy to manage tasks across different cloud environments.
- Context Management: Efficiently manage execution contexts with thread-safe access to shared data.
- Pre- and Post-Execution Actions: Define actions to be executed before and after the main task execution, enabling additional processing and context modification.
- Cloud-Agnostic Execution: Seamlessly execute tasks on AWS Lambda, Alibaba Cloud FC, or local development environments without changing the core task logic.
- Automatic Payload Handling: Simplify task payload handling with built-in JSON parsing and context initialization.

## Architecture
The Cloud Task Executor is built around a modular architecture that includes the following key components:

1. Task: A unit of work that can be executed with a specific context and payload. Tasks are defined using a custom procedural macro for easy registration.
2. Context: A thread-safe, shared data store used to maintain state across task executions. The context can be modified by pre-actions and accessed during task execution.
3.	Before-Action: A closure that modifies the context or payload before the main task execution.
4.	After-Action: A closure that processes the result of the task execution and can modify the final output.
5.	Executor: The core component responsible for managing task registration, context initialization, pre- and post-actions, and task execution across different environments.

## Usage
The Cloud Task Executor can be easily integrated into your Rust projects using the provided crate. To get started, add the following dependency to your Cargo.toml file:

```toml
[dependencies]
cloud_task_executor = "0.1.4"
```

Next, import the crate into your project:
```rust
use cloud_task_executor::*;
```

### Registering Tasks

Tasks are registered using a custom procedural macro cte_task. This macro simplifies the task registration process by automatically generating the necessary boilerplate code. To register a task, define a function with the #[cte_task] attribute and specify the task name and description.

```rust
#[cte_task(name = "my_task")]
async fn my_task(ctx: Context, payload: Value) -> Result<String, String> {
    let sample_value = ctx.get("sample_key").expect("sample_key not found");
    let payload_str = payload.get("payload_key").and_then(Value::as_str).unwrap_or("default_value");
    let runtime = ctx.get(KEY_RUNTIME).unwrap();
    println!("Task running with sample value: {}, payload: {}, runtime {}", sample_value, payload_str, runtime);
    Ok("Task result".to_string())
}
```
### Setting Up the Executor
The executor is set up with context initializers, before-actions, after-actions, and tasks.
```rust
let mut executor = Executor::new();

executor.set_initializer(|ctx| {
ctx.set("sample_key", "sample_value".to_string());
});

executor.set_before_action(|ctx, payload| {
    ctx.set("modified_key", "test".to_string());
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

executor.set_after_action(|ctx, payload, result| {
let context = ctx;
println!("Task executed with payload: {:?}, result: {:?}", payload, result);
result.map(|res| format!("{} - after action", res))
});

executor.set_task(my_task());
```
### Running the Executor
The executor can be run in different environments, including local development, AWS Lambda, and Alibaba Cloud FC.
```rust
#[tokio::main]
async fn main() {
    let result = executor.run().await;
    match result {
        Ok(msg) => println!("{}", msg),
        Err(err) => eprintln!("Error: {}", err),
    }
}
```
### Running the Executor Locally
```shell
cargo run -- --payload '{}'
```

## Conclusion

The Cloud Task Executor is a powerful tool for developers looking to streamline their cloud task execution workflows. With its unified interface, robust context management, and flexible pre- and post-actions, it simplifies the process of writing, registering, and executing tasks in various cloud environments.

This description should provide a clear and comprehensive overview of your project, highlighting its features, architecture, and usage. Let me know if you need any further adjustments or additional details!

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE.txt) file for details.
