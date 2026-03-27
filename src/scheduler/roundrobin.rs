use super::super::cpu::cpu_switch_to;
use super::super::cpu::process::{CpuContext, Process, ProcessState};
use super::super::utils::locked::SpinLock;
use alloc::vec::Vec;

pub(crate) static SCHEDULER: SpinLock<RoundRobin> = SpinLock::new(RoundRobin::new());

pub(crate) struct RoundRobin {
    tasks: Vec<Process>,
    current_task_index: usize,
}

impl RoundRobin {
    const fn new() -> Self {
        RoundRobin {
            current_task_index: 0,
            tasks: Vec::new(),
        }
    }

    pub(crate) fn add_task(&mut self, process: Process) {
        self.tasks.push(process);
    }

    fn drop_dead_tasks(&mut self) {
        self.tasks.retain(|task| !task.is_dead());
        if self.current_task_index >= self.tasks.len() {
            self.current_task_index = 0;
        }
    }

    fn schedule(&mut self) -> Option<(*mut CpuContext, *const CpuContext)> {
        self.drop_dead_tasks();
        if self.tasks.len() <= 1 {
            return None;
        }

        let prev_index = self.current_task_index;
        self.tasks[prev_index].set_state(ProcessState::Ready);

        self.current_task_index += 1;
        if self.current_task_index == self.tasks.len() {
            self.current_task_index = 0;
        }
        let next_index = self.current_task_index;

        self.tasks[next_index].set_state(ProcessState::Running);

        let tasks_ptr = self.tasks.as_mut_ptr();
        let prev_ptr = unsafe { (&mut *tasks_ptr.add(prev_index)).context_ptr_mut() };
        let next_ptr = unsafe { (&*tasks_ptr.add(next_index)).context_ptr() };

        Some((prev_ptr, next_ptr))
    }

    pub(crate) fn kill_current(&mut self) {
        self.tasks[self.current_task_index].kill();
    }
}

pub(crate) fn handle_timer_tick() {
    let switch_ptrs = {
        let mut scheduler = SCHEDULER.lock();
        scheduler.schedule()
    };
    if let Some((prev, next)) = switch_ptrs {
        unsafe {
            cpu_switch_to(prev, next);
        }
    }
}
