pub mod processor;
pub mod scheduler;
pub mod structs;
pub mod thread_pool;
pub mod timer;

use self::timer::Timer;
use crate::fs::{INodeExt, ROOT_INODE};
use crate::timer::now;
use alloc::boxed::Box;
use core::time::Duration;
use lazy_static::lazy_static;
use processor::Processor;
use scheduler::RRScheduler;
use spin::Mutex;
use structs::Thread;
use thread_pool::ThreadPool;

pub type Tid = usize;
pub type ExitCode = usize;

static CPU: Processor = Processor::new();

lazy_static! {
    static ref TIMER: Mutex<Timer> = Mutex::new(Timer::default());
}

pub fn init() {
    let scheduler = RRScheduler::new(1);
    let thread_pool = ThreadPool::new(100, Box::new(scheduler));
    let idle = Thread::new_kernel(Processor::idle_main as usize);
    idle.append_initial_arguments([&CPU as *const Processor as usize, 0, 0]);
    CPU.init(idle, Box::new(thread_pool));

    execute("rust/user_shell", None);

    println!("++++ setup process!   ++++");
}

pub fn execute(path: &str, host_tid: Option<Tid>) -> bool {
    let find_result = ROOT_INODE.lookup(path);
    match find_result {
        Ok(inode) => {
            let data = inode.read_as_vec().unwrap();
            let user_thread = unsafe { Thread::new_user(data.as_slice(), host_tid) };
            CPU.add_thread(user_thread);
            true
        }
        Err(_) => {
            println!("command not found!");
            false
        }
    }
}

pub fn tick(now: Duration) {
    CPU.tick();
    TIMER.lock().tick(now);
}

pub fn run() {
    CPU.run();
}

pub fn exit(code: usize) {
    CPU.exit(code);
}

pub fn yield_now() {
    CPU.yield_now();
}

pub fn wake_up(tid: Tid) {
    CPU.wake_up(tid);
}
pub fn current_tid() -> usize {
    CPU.current_tid()
}

/// Sleep for `duration` time.
pub fn sleep(duration: Duration) {
    let tid = current_tid();
    add_timer(duration, move || wake_up(tid));
    yield_now();
}

/// Add a timer after `interval`.
pub fn add_timer(interval: Duration, callback: impl FnOnce() + Send + Sync + 'static) {
    let deadline = now() + interval;
    TIMER.lock().add(deadline, callback);
}
