use super::super::cpu::cpu_switch_to;
use super::super::cpu::process::{CpuContext, Process, ProcessState};
use super::super::utils::locked::TicketLock;
use alloc::vec::Vec;

const NUM_QUEUES: usize = 5;
const TIME_SLICES: [usize; NUM_QUEUES] = [5, 10, 20, 40, 80];
const BOOST_INTERVAL: u64 = 100;

pub(crate) static SCHEDULER: TicketLock<MLFQ> = TicketLock::new(MLFQ::new());

pub(crate) struct MLFQ {
    queues: [Vec<Process>; NUM_QUEUES],
    graveyard: Vec<Process>,
    boost_counter: u64,
    current_queue: usize,
    total_tasks: usize,
}

impl MLFQ {
    const fn new() -> Self {
        const EMPTY_Q: Vec<Process> = Vec::new();
        MLFQ {
            boost_counter: 0,
            current_queue: 0,
            total_tasks: 0,
            queues: [EMPTY_Q; NUM_QUEUES],
            graveyard: Vec::new(),
        }
    }

    pub(crate) fn add_task(&mut self, process: Process) {
        self.total_tasks += 1;
        self.queues[0].push(process);
    }

    pub(crate) fn kill_current(&mut self) {
        self.queues[self.current_queue][0].kill();
    }

    fn boost_tasks(&mut self) {
        for q in 1..NUM_QUEUES {
            while !self.queues[q].is_empty() {
                let mut task = self.queues[q].remove(0);
                task.set_state(ProcessState::Ready);
                task.set_current_priority(0);
                task.reset_tick_consumed();
                self.queues[0].push(task);
            }
        }
    }

    pub(crate) fn schedule(&mut self) -> Option<(*mut CpuContext, *const CpuContext)> {
        self.graveyard.clear();

        if self.total_tasks <= 1 {
            return None;
        }

        let mut running_task = self.queues[self.current_queue].remove(0);
        let prev_ptr: *mut CpuContext;

        if running_task.is_dead() {
            self.total_tasks -= 1;
            self.graveyard.push(running_task);
            prev_ptr = self.graveyard.last_mut().unwrap().context_ptr_mut();
        } else {
            running_task.set_state(ProcessState::Ready);
            running_task.increment_tick_consumed();

            if running_task.tick_consumed() >= TIME_SLICES[self.current_queue] {
                running_task.reset_tick_consumed();
                let current_priority = running_task.current_priority();
                if current_priority < NUM_QUEUES - 1 {
                    running_task.set_current_priority(current_priority + 1);
                }
            }

            let target_queue = running_task.current_priority();
            self.queues[target_queue].push(running_task);

            let prev_idx = self.queues[target_queue].len() - 1;
            prev_ptr = self.queues[target_queue][prev_idx].context_ptr_mut();
        }

        self.boost_counter += 1;
        if self.boost_counter >= BOOST_INTERVAL {
            self.boost_tasks();
            self.boost_counter = 0;
        }

        let mut next_queue = 0;
        for q in 0..NUM_QUEUES {
            if !self.queues[q].is_empty() {
                next_queue = q;
                break;
            }
        }
        self.current_queue = next_queue;

        self.queues[self.current_queue][0].set_state(ProcessState::Running);
        let next_ptr = self.queues[self.current_queue][0].context_ptr();

        Some((prev_ptr, next_ptr))
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
