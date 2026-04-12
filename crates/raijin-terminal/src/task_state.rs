use raijin_task::SpawnInTerminal;
use smol::channel::Receiver;
use std::process::ExitStatus;

/// State for a terminal that is running a task.
#[derive(Debug)]
pub struct TaskState {
    pub status: TaskStatus,
    pub completion_rx: Receiver<Option<ExitStatus>>,
    pub spawned_task: SpawnInTerminal,
}

/// Status of a terminal tab's task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// The task started but was cancelled or didn't report an exit code.
    Unknown,
    /// The task is currently running.
    Running,
    /// The task completed with a success/failure code.
    Completed { success: bool },
}

impl TaskStatus {
    pub fn register_terminal_exit(&mut self) {
        if self == &Self::Running {
            *self = Self::Unknown;
        }
    }

    pub fn register_task_exit(&mut self, error_code: i32) {
        *self = TaskStatus::Completed {
            success: error_code == 0,
        };
    }
}
