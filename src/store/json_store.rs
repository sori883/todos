use std::cell::RefCell;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use fs2::FileExt;

use crate::error::AppError;
use crate::model::filter::TaskFilter;
use crate::model::stats::Stats;
use crate::model::task::{Status, Task, TaskId};
use crate::store::repository::TaskRepository;
use crate::store::schema::{CURRENT_VERSION, TaskData};

pub struct JsonStore {
    path: PathBuf,
    data: RefCell<Option<TaskData>>,
    batch_mode: RefCell<bool>,
}

impl JsonStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            data: RefCell::new(None),
            batch_mode: RefCell::new(false),
        }
    }

    /// Enable or disable batch mode.
    /// When batch mode is enabled, with_data_mut will not auto-save.
    pub fn set_batch_mode(&self, enabled: bool) {
        *self.batch_mode.borrow_mut() = enabled;
    }

    /// Manually flush (save) data to disk. Used after batch operations.
    /// If no data has been loaded (no operations were performed), this is a no-op.
    pub fn flush(&self) -> Result<(), AppError> {
        if self.data.borrow().is_some() {
            self.save()?;
        }
        Ok(())
    }

    /// Invalidate the in-memory cache so the next operation re-reads from disk.
    pub fn invalidate_cache(&self) {
        *self.data.borrow_mut() = None;
    }

    fn lock_path(&self) -> PathBuf {
        self.path.with_extension("json.lock")
    }

    fn acquire_lock(&self) -> Result<File, AppError> {
        if let Some(parent) = self.lock_path().parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.lock_path())?;

        for attempt in 0..3 {
            match lock_file.try_lock_exclusive() {
                Ok(()) => return Ok(lock_file),
                Err(_) if attempt < 2 => {
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(AppError::FileLock(format!(
                        "Failed to acquire lock after 3 attempts: {e}"
                    )));
                }
            }
        }

        unreachable!()
    }

    fn load(&self) -> Result<(), AppError> {
        if self.data.borrow().is_some() {
            return Ok(());
        }

        if !self.path.exists() {
            *self.data.borrow_mut() = Some(TaskData::empty());
            return Ok(());
        }

        let content = fs::read_to_string(&self.path)?;
        let data: TaskData = serde_json::from_str(&content)?;

        if data.version > CURRENT_VERSION {
            return Err(AppError::SchemaVersionTooNew(data.version, CURRENT_VERSION));
        }

        *self.data.borrow_mut() = Some(data);
        Ok(())
    }

    fn save(&self) -> Result<(), AppError> {
        let data_ref = self.data.borrow();
        let data = data_ref
            .as_ref()
            .ok_or_else(|| AppError::DataFile("No data loaded".to_string()))?;

        let _lock = self.acquire_lock()?;

        let temp_path = self.path.with_extension("json.tmp");
        let file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(&file);
        serde_json::to_writer_pretty(&mut writer, data)?;
        writer.flush()?;
        file.sync_all()?;
        fs::rename(&temp_path, &self.path)?;

        Ok(())
    }

    fn with_data<F, R>(&self, f: F) -> Result<R, AppError>
    where
        F: FnOnce(&TaskData) -> R,
    {
        self.load()?;
        let data_ref = self.data.borrow();
        let data = data_ref
            .as_ref()
            .ok_or_else(|| AppError::DataFile("No data loaded".to_string()))?;
        Ok(f(data))
    }

    fn with_data_mut<F, R>(&self, f: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut TaskData) -> R,
    {
        self.load()?;
        let mut data_ref = self.data.borrow_mut();
        let data = data_ref
            .as_mut()
            .ok_or_else(|| AppError::DataFile("No data loaded".to_string()))?;
        let result = f(data);
        drop(data_ref);
        if !*self.batch_mode.borrow() {
            self.save()?;
        }
        Ok(result)
    }
}

impl TaskRepository for JsonStore {
    fn list(&self, filter: &TaskFilter) -> Result<Vec<Task>, AppError> {
        self.with_data(|data| {
            data.tasks
                .iter()
                .filter(|t| apply_filter(t, filter))
                .cloned()
                .collect()
        })
    }

    fn get(&self, id: TaskId) -> Result<Option<Task>, AppError> {
        self.with_data(|data| data.tasks.iter().find(|t| t.id == id).cloned())
    }

    fn get_by_prefix(&self, prefix: &str) -> Result<Vec<Task>, AppError> {
        self.with_data(|data| {
            let prefix_lower = prefix.to_lowercase();
            data.tasks
                .iter()
                .filter(|t| t.id.to_string().to_lowercase().starts_with(&prefix_lower))
                .cloned()
                .collect()
        })
    }

    fn create(&self, task: Task) -> Result<Task, AppError> {
        let task_clone = task.clone();
        self.with_data_mut(|data| {
            data.tasks.push(task_clone);
        })?;
        Ok(task)
    }

    fn update(&self, task: Task) -> Result<Task, AppError> {
        let task_clone = task.clone();
        self.with_data_mut(|data| {
            if let Some(existing) = data.tasks.iter_mut().find(|t| t.id == task_clone.id) {
                *existing = task_clone;
            }
        })?;
        Ok(task)
    }

    fn delete(&self, id: TaskId) -> Result<Option<Task>, AppError> {
        self.with_data_mut(|data| {
            if let Some(pos) = data.tasks.iter().position(|t| t.id == id) {
                Some(data.tasks.remove(pos))
            } else {
                None
            }
        })
    }

    fn get_children(&self, parent_id: TaskId) -> Result<Vec<Task>, AppError> {
        self.with_data(|data| {
            data.tasks
                .iter()
                .filter(|t| t.parent_id == Some(parent_id))
                .cloned()
                .collect()
        })
    }

    fn stats(&self, filter: &TaskFilter) -> Result<Stats, AppError> {
        self.with_data(|data| {
            let filtered: Vec<&Task> = data
                .tasks
                .iter()
                .filter(|t| apply_filter(t, filter))
                .collect();

            let mut by_status = std::collections::HashMap::new();
            let mut by_priority = std::collections::HashMap::new();
            let mut by_label = std::collections::HashMap::new();
            let mut by_project = std::collections::HashMap::new();
            let mut by_creator = std::collections::HashMap::new();

            let mut todo = 0;
            let mut in_progress = 0;
            let mut done = 0;
            let mut cancelled = 0;

            for task in &filtered {
                match task.status {
                    Status::Todo => todo += 1,
                    Status::InProgress => in_progress += 1,
                    Status::Done => done += 1,
                    Status::Cancelled => cancelled += 1,
                }

                *by_status.entry(task.status.to_string()).or_insert(0) += 1;

                *by_priority.entry(task.priority.to_string()).or_insert(0) += 1;

                if let Some(ref label) = task.label {
                    *by_label.entry(label.clone()).or_insert(0) += 1;
                }

                if let Some(ref project) = task.project {
                    *by_project.entry(project.clone()).or_insert(0) += 1;
                }

                *by_creator.entry(task.created_by.to_string()).or_insert(0) += 1;
            }

            Stats {
                total: filtered.len(),
                todo,
                in_progress,
                done,
                cancelled,
                by_status,
                by_priority,
                by_label,
                by_project,
                by_creator,
            }
        })
    }
}

fn apply_filter(task: &Task, filter: &TaskFilter) -> bool {
    if let Some(ref status) = filter.status
        && task.status != *status
    {
        return false;
    }

    if let Some(ref priority) = filter.priority
        && task.priority != *priority
    {
        return false;
    }

    if let Some(ref label) = filter.label {
        match &task.label {
            Some(task_label) if task_label == label => {}
            _ => return false,
        }
    }

    if let Some(ref created_by) = filter.created_by
        && task.created_by != *created_by
    {
        return false;
    }

    if let Some(ref project) = filter.project {
        match &task.project {
            Some(task_project) if task_project == project => {}
            _ => return false,
        }
    }

    if let Some(ref parent_filter) = filter.parent_id
        && task.parent_id != *parent_filter
    {
        return false;
    }

    if !filter.include_done && task.status == Status::Done {
        return false;
    }

    if !filter.include_cancelled && task.status == Status::Cancelled {
        return false;
    }

    true
}
