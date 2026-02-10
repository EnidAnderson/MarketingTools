use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Represents a scheduled task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub task_id: String,
    pub frequency: String, // e.g., "daily", "weekly", "monthly"
    pub last_run: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub data: serde_json::Value, // Using serde_json::Value for flexible data storage
}

impl ScheduledTask {
    /// Creates a new `ScheduledTask`.
    pub fn new(
        task_id: String,
        frequency: String,
        last_run: Option<DateTime<Utc>>,
        enabled: bool,
        data: serde_json::Value,
    ) -> Self {
        Self {
            task_id,
            frequency,
            last_run,
            enabled,
            data,
        }
    }
}

/// Manages a registry of scheduled tasks, persisting them to a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskRegistry {
    #[serde(skip)] // Don't serialize this field
    storage_file: PathBuf,
    tasks: HashMap<String, ScheduledTask>,
}

impl ScheduledTaskRegistry {
    /// Creates a new registry and loads tasks from the specified file.
    pub fn new<P: AsRef<Path>>(storage_file: P) -> Self {
        let storage_file_path = storage_file.as_ref().to_path_buf();
        let tasks = Self::load_tasks(&storage_file_path).unwrap_or_default();
        Self {
            storage_file: storage_file_path,
            tasks,
        }
    }

    /// Loads scheduled tasks from the storage file.
    fn load_tasks(storage_file: &PathBuf) -> Result<HashMap<String, ScheduledTask>, io::Error> {
        if storage_file.exists() {
            let contents = fs::read_to_string(storage_file)?;
            if contents.trim().is_empty() {
                Ok(HashMap::new())
            } else {
                let task_list: Vec<ScheduledTask> = serde_json::from_str(&contents)?;
                Ok(task_list
                    .into_iter()
                    .map(|t| (t.task_id.clone(), t))
                    .collect())
            }
        } else {
            Ok(HashMap::new())
        }
    }

    /// Saves scheduled tasks to the storage file.
    fn save_tasks(&self) -> Result<(), io::Error> {
        let task_list: Vec<&ScheduledTask> = self.tasks.values().collect();
        let contents = serde_json::to_string_pretty(&task_list)?;
        fs::write(&self.storage_file, contents)?;
        Ok(())
    }

    /// Adds or updates a scheduled task.
    pub fn add_schedule(&mut self, task: ScheduledTask) -> Result<(), io::Error> {
        self.tasks.insert(task.task_id.clone(), task);
        self.save_tasks()
    }

    /// Retrieves a specific scheduled task by its ID.
    pub fn get_schedule(&self, task_id: &str) -> Option<&ScheduledTask> {
        self.tasks.get(task_id)
    }

    /// Retrieves a list of scheduled tasks, optionally filtered by frequency and enabled status.
    pub fn get_schedules(
        &self,
        frequency: Option<&str>,
        enabled_only: bool,
    ) -> Vec<&ScheduledTask> {
        self.tasks
            .values()
            .filter(|task| {
                if enabled_only && !task.enabled {
                    return false;
                }
                if let Some(freq) = frequency {
                    if task.frequency != freq {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    /// Updates the `last_run` timestamp for a given task.
    pub fn update_last_run(
        &mut self,
        task_id: &str,
        last_run: DateTime<Utc>,
    ) -> Result<(), io::Error> {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.last_run = Some(last_run);
            self.save_tasks()
        } else {
            // If the task is not found, we might want to return an error or do nothing.
            // For now, let's return an error.
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Task with ID '{}' not found.", task_id),
            ))
        }
    }

    /// Removes a scheduled task by its ID.
    pub fn remove_schedule(&mut self, task_id: &str) -> Result<(), io::Error> {
        if self.tasks.remove(task_id).is_some() {
            self.save_tasks()
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Task with ID '{}' not found.", task_id),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Timelike, Utc}; // Import Timelike
    use serde_json::json;
    use tempfile::NamedTempFile;

    fn setup_registry() -> (ScheduledTaskRegistry, PathBuf) {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();
        (ScheduledTaskRegistry::new(&path), path)
    }

    #[test]
    fn test_add_and_get_schedule() {
        let (mut registry, _temp_path) = setup_registry();
        let task_id = "test_task_1".to_string();
        let task = ScheduledTask::new(
            task_id.clone(),
            "daily".to_string(),
            None,
            true,
            json!({"key": "value"}),
        );

        registry.add_schedule(task.clone()).unwrap();

        let retrieved_task = registry.get_schedule(&task_id).unwrap();
        assert_eq!(retrieved_task, &task);
    }

    #[test]
    fn test_get_schedules_filtered() {
        let (mut registry, _temp_path) = setup_registry();

        let task1 =
            ScheduledTask::new("t1".to_string(), "daily".to_string(), None, true, json!({}));
        let task2 = ScheduledTask::new(
            "t2".to_string(),
            "weekly".to_string(),
            None,
            true,
            json!({}),
        );
        let task3 = ScheduledTask::new(
            "t3".to_string(),
            "daily".to_string(),
            None,
            false,
            json!({}),
        );

        registry.add_schedule(task1.clone()).unwrap();
        registry.add_schedule(task2.clone()).unwrap();
        registry.add_schedule(task3.clone()).unwrap();

        let daily_tasks = registry.get_schedules(Some("daily"), true);
        assert_eq!(daily_tasks.len(), 1);
        assert_eq!(daily_tasks[0], &task1);

        let all_enabled = registry.get_schedules(None, true);
        assert_eq!(all_enabled.len(), 2);
        assert!(all_enabled.contains(&&task1));
        assert!(all_enabled.contains(&&task2));

        let all_tasks = registry.get_schedules(None, false);
        assert_eq!(all_tasks.len(), 3);
    }

    #[test]
    fn test_update_last_run() {
        let (mut registry, _temp_path) = setup_registry();
        let task_id = "update_task".to_string();
        let task = ScheduledTask::new(task_id.clone(), "daily".to_string(), None, true, json!({}));
        registry.add_schedule(task).unwrap();

        let now = Utc::now().with_nanosecond(0).unwrap(); // Truncate nanoseconds for comparison
        registry.update_last_run(&task_id, now).unwrap();

        let updated_task = registry.get_schedule(&task_id).unwrap();
        assert_eq!(updated_task.last_run, Some(now));
    }

    #[test]
    fn test_remove_schedule() {
        let (mut registry, _temp_path) = setup_registry();
        let task_id = "remove_task".to_string();
        let task = ScheduledTask::new(task_id.clone(), "daily".to_string(), None, true, json!({}));
        registry.add_schedule(task).unwrap();

        assert!(registry.get_schedule(&task_id).is_some());
        registry.remove_schedule(&task_id).unwrap();
        assert!(registry.get_schedule(&task_id).is_none());
    }

    #[test]
    fn test_persistence() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();

        // Add tasks to a registry and save
        {
            let mut registry = ScheduledTaskRegistry::new(&path);
            let task1 = ScheduledTask::new(
                "p1".to_string(),
                "daily".to_string(),
                Some(Utc::now().with_nanosecond(0).unwrap()),
                true,
                json!({"prio": 1}),
            );
            let task2 = ScheduledTask::new(
                "p2".to_string(),
                "weekly".to_string(),
                None,
                false,
                json!({}),
            );
            registry.add_schedule(task1.clone()).unwrap();
            registry.add_schedule(task2.clone()).unwrap();
        } // Registry goes out of scope and saves

        // Load tasks into a new registry
        let loaded_registry = ScheduledTaskRegistry::new(&path);
        assert_eq!(loaded_registry.tasks.len(), 2);
        assert!(loaded_registry.get_schedule("p1").is_some());
        assert!(loaded_registry.get_schedule("p2").is_some());
        assert_eq!(
            loaded_registry.get_schedule("p1").unwrap().frequency,
            "daily"
        );
    }

    #[test]
    fn test_empty_file_load() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();
        fs::write(&path, "").unwrap(); // Create an empty file

        let registry = ScheduledTaskRegistry::new(&path);
        assert!(registry.tasks.is_empty());
    }

    #[test]
    fn test_file_not_found() {
        let path = PathBuf::from("non_existent_file.json");
        let registry = ScheduledTaskRegistry::new(&path);
        assert!(registry.tasks.is_empty());
    }
}
