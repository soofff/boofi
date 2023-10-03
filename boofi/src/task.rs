use std::sync::Arc;
use serde::Serialize;
use serde_json::{to_value, Value};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::apps::AppBuilders;
use crate::apps::prelude::Deserialize;
use crate::error::{Erro, Resul};
use crate::system::System;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskStatus {
    Created,
    Running,
    Finished,
    Failed,
}

/// Represents a task with id, in/output, app name and status
#[derive(Serialize, Deserialize)]
pub(crate) struct Task {
    id: usize,
    app_name: String,
    status: TaskStatus,
    app_input: Value,
    #[serde(skip)]
    app: Option<AppBuilders>,
    app_output: Option<Value>,
    app_error: Option<String>,
}

impl Task {
    pub(crate) fn id(&self) -> usize { self.id }
}

/// Manages all tasks
/// All tasks (apps) running asynchronous
pub(crate) struct TaskController {
    tasks: Arc::<Mutex::<Vec<Task>>>,
    last_id: usize,
}

impl Default for TaskController {
    fn default() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(vec![])),
            last_id: 0,
        }
    }
}

impl TaskController {
    /// Generate a new task and starts the app asynchronously
    /// In and output is stored in json format
    pub(crate) async fn new_task(&mut self, mut app: AppBuilders, value: Value, system: System) -> Resul<Value> {
        log::trace!("[TASK] creating new task with app {}",  app.name());

        let mut tasks = self.tasks.lock().await;
        let id = self.last_id + 1;

        let task = Task {
            id,
            app_name: app.name().into(),
            app_input: value.clone(),
            app: None,
            app_output: None,
            status: TaskStatus::Created,
            app_error: None,
        };

        let task_value = to_value(&task)?;
        tasks.push(task);

        log::debug!("[TASK] new task {} created", id);

        self.last_id = id;

        let tasks = self.tasks.clone();

        let j: JoinHandle<Resul<()>> = tokio::spawn(async move {
            let index = id - 1;
            log::trace!("[TASK] task {} spawned", id);

            tasks.lock().await.get_mut(index).ok_or(Erro::TaskInvalidIndex)?.status = TaskStatus::Running;
            log::debug!("[TASK] task {} running", id);

            let a = app.run(value, &system).await;

            let result = a;
            log::debug!("[TASK] task {} run done", id);

            let mut tasks_unlocked = tasks.lock().await;
            let mut task = tasks_unlocked.get_mut(index).ok_or(Erro::TaskInvalidIndex)?;

            match result {
                Ok(result) => {
                    log::info!("[TASK] task {} run successfully", id);
                    task.app_output = Some(to_value(result)?);
                    task.status = TaskStatus::Finished;
                }
                Err(error) => {
                    log::error!("[TASK] task {} failed", id);
                    task.app_error = Some(format!("{:?}", error));
                    task.status = TaskStatus::Failed;
                }
            };

            task.app = Some(app);
            Ok(())
        });

        drop(j);

        Ok(task_value)
    }

    /// Returns all tasks in a mutex context
    pub(crate) fn tasks(&self) -> Arc<Mutex<Vec<Task>>> {
        self.tasks.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use serde_json::{from_value, json};
    use crate::apps::ls::LsBuilder;
    use crate::apps::AppBuilders;
    use crate::task::{Task, TaskController, TaskStatus};
    use crate::utils::test::system_user;

    #[tokio::test]
    async fn new_task() {
        let mut tk = TaskController::default();

        let app_builder = AppBuilders::LsBuilder(LsBuilder::default());
        let app = app_builder;
        let input = json!({"path": "/"});
        let result = tk.new_task(app.clone(), input.clone(), system_user().await).await.unwrap();

        let t1: Task = from_value(result).unwrap();

        assert_eq!(t1.status, TaskStatus::Created);
        assert_eq!(t1.id, 1);
        assert_eq!(t1.app_input, input);
        assert_eq!(t1.app_name, app.name().to_string());

        tokio::time::sleep(Duration::from_secs(5)).await;

        let t = tk.tasks();
        let tasks = t.lock().await;
        assert_eq!(tasks[0].status, TaskStatus::Finished);
        assert!(tasks[0].app_output.as_ref().unwrap().is_array())
    }

    #[tokio::test]
    async fn new_task_failed() {
        let mut tk = TaskController::default();

        let app_builder = AppBuilders::LsBuilder(LsBuilder::default());
        let app = app_builder;
        let input = json!({"invalid": "/"});
        tk.new_task(app, input.clone(), system_user().await).await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;

        let t = tk.tasks();
        let tasks = t.lock().await;

        assert_eq!(tasks[0].status, TaskStatus::Failed);
        dbg!(&tasks[0].app_error);
        assert!(tasks[0].app_error.is_some());
    }
}
