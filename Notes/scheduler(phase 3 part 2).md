# Scheduler — Phase 3 Part 2 Notes

**OSTEP Chapters:** 7, 8, 9 (core for this phase: 8)
**What I built:** Preemptive scheduling with Round Robin and MLFQ, plus hard-earned bug fixes for context switching correctness
**Files:**

- `src/scheduler/mod.rs` — scheduler module wiring
- `src/scheduler/roundrobin.rs` — baseline Round Robin
- `src/scheduler/mlfq.rs` — MLFQ with priorities, demotion, boosts, and graveyard
- `src/cpu/process.rs` — process startup/trampoline and per-task CPU context
- `src/cpu/switch.s` — low-level context switch (`cpu_switch_to`)
- `src/cpu/vectors.s` — interrupt entry/exit and ELR/SPSR save-restore
- `src/drivers/timer.rs` — periodic tick source that drives preemption
- `src/main.rs` — test tasks and `sys_exit()` behavior

---

## Big Picture First

Phase 3 Part 1 gave us the machinery to receive timer interrupts and switch contexts.
Phase 3 Part 2 makes that machinery actually schedule tasks fairly and safely.

Before this phase, task switching existed as a mechanism.
After this phase, task switching follows policy (who runs next, for how long, and why).

This phase is where OSTEP moves from **"what is a process"** to **"how do we share CPU time correctly"**.

---

## Code Sync Update (2026-03-27)

This section reflects the current implementation and should be treated as source of truth.

### Current timer behavior (`src/drivers/timer.rs`)

- `cntfrq_el0` is read at runtime.
- `ticks = frequency / 10`.
- So timer fires about 10 times per second (roughly every 100 ms).

### Current MLFQ behavior (`src/scheduler/mlfq.rs`)

- Number of queues: `NUM_QUEUES = 5`
- Time slices per queue: `[5, 10, 20, 40, 80]` ticks
- Priority boost interval: `BOOST_INTERVAL = 100`
- Dead tasks are not immediately dropped from memory.
- Dead tasks are moved to `graveyard: Vec<Process>` to preserve a safe context save target.

### Current process startup behavior (`src/cpu/process.rs`)

- New task entry point is placed in `x19`.
- `x30` points to `task_startup` (kernel trampoline), not directly to user function.
- `task_startup` forcefully unmasks interrupts (`msr daifclr, #0b1111`) and then `blr x19`.

### Current interrupt return behavior (`src/cpu/vectors.s`)

- On interrupt entry, ELR/SPSR are copied from hardware registers to task stack.
- On interrupt exit, ELR/SPSR are restored from stack back into hardware registers.
- `eret` then returns to the correct instruction for that specific task.

---

## OSTEP Theory You Actually Used

## Chapter 7: Scheduling Introduction

Chapter 7 asks: what should a scheduler optimize?

Main metrics:

1. **Turnaround time**: submission to completion.
2. **Response time**: submission to first run (important for interactive jobs).
3. **Fairness**: no task should starve forever.
4. **Throughput**: how much total work finishes over time.

No scheduler can maximize all metrics simultaneously.
This is the central tradeoff of scheduling.

Simple example:

- If you always run shortest jobs first, average turnaround improves.
- But long jobs can starve if short jobs keep arriving.

So scheduling is never "best overall", it is always "best for a goal".

## Chapter 8: Multi-Level Feedback Queue (MLFQ)

This chapter introduces practical dynamic priority scheduling.

Why MLFQ exists:

- We usually do not know job length in advance.
- Interactive jobs should feel snappy.
- CPU hogs should not dominate.

MLFQ rules (conceptually):

1. New jobs start at highest priority.
2. If a job uses its full slice, it is probably CPU-bound -> demote it.
3. If a job yields early/often, it behaves interactive -> keep it high.
4. Periodic boost moves everyone up to prevent starvation.

Your implementation maps directly to this:

- `current_priority` tracks level.
- `tick_consumed` tracks slice usage.
- Demotion occurs when `tick_consumed >= TIME_SLICES[level]`.
- Global `boost_counter` periodically triggers `boost_tasks()`.

## Chapter 9: Proportional Share (Lottery/Stride)

Chapter 9 introduces a different philosophy: not strict queues, but weighted CPU shares.

Idea:

- Give each task tickets (weight).
- Scheduler picks winner randomly (lottery) or deterministically (stride).
- Over time, CPU time approaches ticket ratio.

Why this is useful:

- Easier resource share guarantees (e.g., task A gets ~2x task B).
- Better for mixed workloads needing explicit weighting.

Why you still used MLFQ now:

- MLFQ is easier to reason about for interactive-vs-CPU-bound behavior.
- MLFQ aligns naturally with your existing timer tick model and process states.
- Weighted schedulers are a great next-phase upgrade once current correctness is rock solid.

---

## Why One Scheduler Is Better Than Another

No single scheduler is universally best. Better means better for your target workload.

## FCFS (First-Come, First-Served)

Good:

- Extremely simple.
- Low scheduler overhead.

Bad:

- Convoy effect: one long job blocks many short ones.
- Terrible response time for interactive jobs.

Use when:

- Batch workloads, simplicity first.

## Round Robin (RR)

Good:

- Fair turn-taking.
- Good baseline responsiveness.
- Easy to implement and debug.

Bad:

- Context switch overhead if quantum is too small.
- Treats all jobs similarly (no workload awareness).

Use when:

- You need a strong baseline and predictable fairness.

## MLFQ

Good:

- Adapts to behavior automatically.
- Interactive tasks stay responsive.
- CPU hogs still run, but at lower priority.

Bad:

- More complex than RR.
- Easy to get bugs in queue movement, accounting, and edge cases.
- Requires careful anti-starvation policy (boosting).

Use when:

- Mixed workloads (interactive + background compute).

## Lottery / Stride (Proportional Share)

Good:

- Expressive fairness (weighted shares).
- Natural for service-level balancing.

Bad:

- More bookkeeping.
- Lottery has randomness variance.
- Stride needs careful precision/overflow handling.

Use when:

- You want explicit CPU share guarantees.

Short practical ranking for your current kernel stage:

1. RR is best for validating context switch correctness.
2. MLFQ is best for real mixed-task behavior now.
3. Proportional share is best next if you want weighted fairness.

---

## The Two Big Problems You Hit (And Why They Hurt)

You saw what looked like one weird bug, but it was actually two independent issues:

1. timing/preemption visibility issue,
2. fatal context overwrite issue.

## 1) The Speed-of-Light Problem ("Why no preemption?")

What happened:

- Early test loops were too short.
- Tasks completed before first visible timer-driven preemption.
- Logs looked like scheduler was broken even when interrupt path was fine.

Why:

- CPU executes NOP loops very fast.
- If task lifetime is shorter than tick interval, task can run to completion before next timer interrupt.

Fix:

- Increase loop work significantly (you moved to `500_000_000` loops in test tasks).
- Use a frequent enough timer tick.

Result:

- Preemption becomes observable in logs.
- RR/MLFQ behavior is now visible instead of tasks "teleport finishing".

## 2) Ghost Task Overwrite ("Why did Task B disappear?")

This one is legendary because it looks random, but it is deterministic corruption.

### What happened step-by-step

1. Task A finished and marked itself dead.
2. Scheduler removed dead tasks from active queue before switching.
3. Queue compaction shifted Task B into the position scheduler then treated as running-task metadata.
4. Scheduler passed Task B context pointer as `prev` into `cpu_switch_to(prev, next)`.
5. But CPU was physically still executing Task A’s exit/sleep loop at that instant.
6. `cpu_switch_to` saved current CPU registers (Task A state) into `prev` (Task B memory).
7. Task B boot context was overwritten by Task A dead-loop context.
8. Later Task B resumed directly into sleep/exit behavior and never printed anything.

Why this is fatal:

- Context save is raw register dump.
- Wrong destination pointer means silent memory corruption of another task’s brain.

### The fix: Graveyard pattern

Key principle:

- Never free/remove the just-dying running task context before the switch-save happens.

Implementation:

- Removed eager dead-drop behavior from active scheduling path.
- Added `graveyard: Vec<Process>`.
- If running task is dead, move that process object to graveyard and use that stable memory as `prev_ptr` sink.

Why this works:

- CPU always gets a valid safe place to spill outgoing registers.
- No living task context gets clobbered.

---

## The Three Major Correctness Bugs You Solved

## Bug A: Locking Deadlock Around `cpu_switch_to`

### Problem

Your `SpinLock` disables interrupts on lock acquire (Pi5 path), and restores state on guard drop.

Old flow (bad):

1. lock scheduler,
2. decide next task,
3. call `cpu_switch_to` while lock guard still alive.

Why broken:

- `cpu_switch_to` does not return to the same control flow immediately.
- Guard drop never runs at that point.
- Interrupts stay masked.
- Timer stops preempting.
- System appears stuck in one task forever.

### Fix

Split selection from switch:

- `schedule()` returns `(prev_ptr, next_ptr)` only.
- `handle_timer_tick()` locks in a scoped block, collects pointers, exits scope.
- Lock guard drops before switch, restoring interrupt mask.
- Then call `cpu_switch_to` outside lock scope.

Current pattern (correct):

- scoped lock for bookkeeping,
- unlocked low-level context switch.

This is exactly what your `src/scheduler/mlfq.rs` and `src/scheduler/roundrobin.rs` now do.

## Bug B: Shared ELR Whiteboard Corruption (`vectors.s`)

### Simple analogy

ELR is like a single whiteboard where CPU writes "where to continue after interrupt".

If two tasks share one hardware whiteboard and you do not back it up per-task, one task can erase another task’s return address.

### Register meaning

- `ELR_EL1` / `ELR_EL2` / `ELR_EL3`:
  Exception Link Register for each privilege level.
  Stores return program counter used by `eret`.

- `SPSR_EL1` / `SPSR_EL2` / `SPSR_EL3`:
  Saved Program Status Register.
  Stores interrupt mask bits, condition flags, execution state that `eret` restores.

- `CurrentEL`:
  Current exception level (EL1/EL2/EL3).

- `eret`:
  Return from exception using ELR+SPSR.

### Problem sequence

1. Task Main interrupted -> ELR contains Main return address.
2. Scheduler switches to Task A.
3. Task A interrupted -> ELR overwritten with Task A return address.
4. Later returning to Main with stale/incorrect ELR leads to wrong jump target.

### Fix in `vectors.s`

On entry:

- read ELR/SPSR for current EL,
- push them onto current task stack (`stp x1, x2, [sp, #-16]!`).

On exit:

- pop ELR/SPSR backup (`ldp x1, x2, [sp], #16`),
- write back to correct ELR/SPSR registers,
- `eret`.

Why this works:

- Each task now carries its own return-address snapshot on its own stack.
- Shared hardware registers are treated as temporary scratch, not durable storage.

## Bug C: User Code Should Not Control Interrupt Policy (`process.rs`)

### Problem

New tasks had no prior interrupt frame.
Bootstrap used `ret` into task entry.
Initially this required user task code to manually re-enable interrupts.

Why this is bad:

- Violates privilege separation.
- Any buggy task forgetting that line can freeze timer/preemption.
- Kernel safety should not depend on user task discipline.

### Fix: Kernel Trampoline (`task_startup`)

Implementation:

1. put user entry in `x19`,
2. set `x30` to `task_startup`,
3. `ret` lands in trampoline first,
4. trampoline executes `msr daifclr, #0b1111`,
5. trampoline `blr x19` to user function,
6. if user returns, park safely in `wfe` loop.

Register meaning here:

- `x30`: link register / return target for `ret`.
- `x19`: callee-saved GP register used to stash true user entry pointer.
- `daif`: interrupt mask register bits.
- `msr daifclr, #0b1111`: clear DAIF mask bits (unmask interrupts).

Why this is correct:

- Kernel enforces hardware state policy.
- User code remains sandboxed from low-level interrupt setup responsibilities.

---

## MLFQ Walkthrough in Plain Terms

Think of five lines at a theme park:

- Queue 0 is VIP (highest priority).
- Queue 4 is slow lane (lowest priority).

Rules:

1. Everyone enters VIP line first.
2. If you keep hogging ride time, you get moved to lower-priority line.
3. Every so often, everyone gets one reset back up (priority boost).

In code:

1. Remove currently running task from queue head.
2. If dead -> move to graveyard.
3. Else mark ready, increment consumed ticks.
4. If slice used up -> maybe demote.
5. Push task into target queue tail.
6. Maybe perform global boost every `BOOST_INTERVAL` ticks.
7. Pick next non-empty highest-priority queue.
8. Mark selected task running.
9. Return `(prev_ptr, next_ptr)` for low-level switch.

This separation of policy and mechanism is important:

- Policy decides who should run.
- Mechanism (`cpu_switch_to`) performs raw register transfer.

---

## Round Robin vs MLFQ in Your Project

Round Robin (`src/scheduler/roundrobin.rs`):

- Cycles through tasks in fixed order.
- Good for sanity checks.
- Easier to prove correctness.

MLFQ (`src/scheduler/mlfq.rs`):

- Priority-aware dynamic behavior.
- Better user-visible responsiveness.
- More edge cases (dead tasks, boosts, demotions, pointer lifetime).

Why MLFQ became your main path:

- It reflects real OS behavior better for mixed workloads.
- Your bug fixes now make it robust enough for next phases.

---

## All Register Meanings (Simple Cheat Sheet)

- `sp`: Stack Pointer (current top of stack for active task).
- `x19`..`x30`: general/callee-saved registers preserved across function calls.
- `x30` (LR): link register used by `ret`.
- `ELR_ELx`: where CPU should return after exception at level x.
- `SPSR_ELx`: saved processor flags/state for exception return.
- `DAIF`: interrupt mask bits (Debug, SError, IRQ, FIQ masks).
- `cntfrq_el0`: hardware timer base frequency (ticks/sec).
- `cntv_tval_el0`: virtual timer countdown reload value.
- `cntv_ctl_el0`: virtual timer enable/control register.
- `CurrentEL`: current privilege level.

---

## What Bugs This Phase Taught Me

1. Correct scheduler math is not enough; pointer lifetime and memory ownership are equally critical.
2. Never hold locks across non-local control transfer points (context switch, trap return, long jumps).
3. Hardware registers are shared global state; treat them as temporary and always snapshot per-task state.
4. Kernel invariants must be enforced in kernel code, never delegated to user tasks.
5. Most "random" scheduler bugs are deterministic state corruption with delayed symptoms.

---

## What Is Now Stable

1. Timer-driven preemption is observable and repeatable.
2. RR and MLFQ both follow lock-drop-before-switch pattern.
3. Dead task handling no longer corrupts live task context (graveyard fix).
4. ELR/SPSR per-task save/restore prevents return-address cross-talk.
5. New tasks start through kernel trampoline with interrupt policy enforced by kernel.

This is a strong base for the next component (syscalls/process control expansion, or user-mode isolation with MMU).

---

## If I Explain This to "Future Me" in 30 Seconds

We built a real preemptive scheduler path and then debugged the dangerous parts that textbooks mention but do not make you feel until you ship code.
The two killers were timing illusions and context-pointer corruption.
The final architecture is now clean:

1. timer tick enters kernel,
2. scheduler computes next task under lock,
3. lock is dropped,
4. assembly switches context,
5. vectors save/restore ELR/SPSR per task,
6. new tasks start via kernel trampoline,
7. dead tasks go to graveyard so context save never writes into live task memory.

That is the difference between "it usually runs" and "it is scheduler-correct."
