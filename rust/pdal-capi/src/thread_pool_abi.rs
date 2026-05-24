use crate::error::{ffi_catch, set_last_error};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

type TaskRun = unsafe extern "C" fn(*mut c_void);
type TaskDrop = unsafe extern "C" fn(*mut c_void);

struct Task {
    data: *mut c_void,
    run: TaskRun,
    drop: TaskDrop,
}

unsafe impl Send for Task {}

impl Task {
    fn run(self) {
        unsafe {
            (self.run)(self.data);
        }
        std::mem::forget(self);
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.data);
        }
    }
}

struct State {
    running: bool,
    queue: VecDeque<Task>,
    outstanding: usize,
}

struct Inner {
    state: Mutex<State>,
    produce: Condvar,
    consume: Condvar,
    queue_size: i64,
}

#[allow(non_camel_case_types)]
pub struct pdal_thread_pool_t {
    inner: Arc<Inner>,
    threads: Vec<JoinHandle<()>>,
    num_threads: usize,
}

impl pdal_thread_pool_t {
    fn new(num_threads: usize, queue_size: i64) -> Self {
        let inner = Arc::new(Inner {
            state: Mutex::new(State {
                running: false,
                queue: VecDeque::new(),
                outstanding: 0,
            }),
            produce: Condvar::new(),
            consume: Condvar::new(),
            queue_size,
        });
        let mut pool = Self {
            inner,
            threads: Vec::new(),
            num_threads: num_threads.max(1),
        };
        pool.go();
        pool
    }

    fn go(&mut self) {
        let mut state = self.inner.state.lock().unwrap();
        if state.running {
            return;
        }
        state.running = true;
        drop(state);
        for _ in 0..self.num_threads {
            let inner = Arc::clone(&self.inner);
            self.threads.push(thread::spawn(move || worker(inner)));
        }
    }

    fn join(&mut self) {
        {
            let mut state = self.inner.state.lock().unwrap();
            if !state.running {
                return;
            }
            state.running = false;
        }
        self.inner.consume.notify_all();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }

    fn stop(&mut self) {
        self.join();
        self.clear_tasks();
    }

    fn clear_tasks(&mut self) {
        let mut state = self.inner.state.lock().unwrap();
        state.queue.clear();
        self.inner.produce.notify_all();
    }

    fn await_empty(&self) {
        let mut state = self.inner.state.lock().unwrap();
        while state.outstanding != 0 || !state.queue.is_empty() {
            state = self.inner.produce.wait(state).unwrap();
        }
    }

    fn add(&self, task: Task) -> Result<(), (Task, String)> {
        let mut state = self.inner.state.lock().unwrap();
        if !state.running {
            return Err((
                task,
                "Attempted to add a task to a stopped ThreadPool".to_string(),
            ));
        }
        while self.inner.queue_size >= 0 && state.queue.len() >= self.inner.queue_size as usize {
            state = self.inner.produce.wait(state).unwrap();
        }
        state.queue.push_back(task);
        drop(state);
        self.inner.consume.notify_all();
        Ok(())
    }
}

fn worker(inner: Arc<Inner>) {
    loop {
        let task = {
            let mut state = inner.state.lock().unwrap();
            loop {
                if let Some(task) = state.queue.pop_front() {
                    state.outstanding += 1;
                    inner.produce.notify_all();
                    break task;
                }
                if !state.running {
                    return;
                }
                state = inner.consume.wait(state).unwrap();
            }
        };
        task.run();
        let mut state = inner.state.lock().unwrap();
        state.outstanding -= 1;
        inner.produce.notify_all();
    }
}

#[no_mangle]
pub extern "C" fn pdal_thread_pool_create(
    num_threads: usize,
    queue_size: i64,
) -> *mut pdal_thread_pool_t {
    ffi_catch(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(pdal_thread_pool_t::new(num_threads, queue_size)))
    })
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_destroy(handle: *mut pdal_thread_pool_t) {
    if !handle.is_null() {
        let mut pool = Box::from_raw(handle);
        pool.stop();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_go(handle: *mut pdal_thread_pool_t) {
    ffi_catch((), || {
        if let Some(pool) = handle.as_mut() {
            pool.go();
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_join(handle: *mut pdal_thread_pool_t) {
    ffi_catch((), || {
        if let Some(pool) = handle.as_mut() {
            pool.join();
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_stop(handle: *mut pdal_thread_pool_t) {
    ffi_catch((), || {
        if let Some(pool) = handle.as_mut() {
            pool.stop();
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_clear_tasks(handle: *mut pdal_thread_pool_t) {
    ffi_catch((), || {
        if let Some(pool) = handle.as_mut() {
            pool.clear_tasks();
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_await(handle: *const pdal_thread_pool_t) {
    ffi_catch((), || {
        if let Some(pool) = handle.as_ref() {
            pool.await_empty();
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_resize(
    handle: *mut pdal_thread_pool_t,
    num_threads: usize,
) {
    ffi_catch((), || {
        if let Some(pool) = handle.as_mut() {
            pool.join();
            pool.num_threads = num_threads;
            pool.go();
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_num_threads(handle: *const pdal_thread_pool_t) -> usize {
    ffi_catch(0, || handle.as_ref().map_or(0, |pool| pool.num_threads))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_thread_pool_add(
    handle: *const pdal_thread_pool_t,
    data: *mut c_void,
    run: Option<TaskRun>,
    drop: Option<TaskDrop>,
) -> bool {
    ffi_catch(false, || {
        let Some(pool) = handle.as_ref() else {
            set_last_error("null ThreadPool handle");
            return false;
        };
        let (Some(run), Some(drop)) = (run, drop) else {
            set_last_error("null ThreadPool task callback");
            return false;
        };
        match pool.add(Task { data, run, drop }) {
            Ok(()) => true,
            Err((task, message)) => {
                std::mem::forget(task);
                set_last_error(message);
                false
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    unsafe extern "C" fn increment_and_drop(data: *mut c_void) {
        let counter = Box::from_raw(data.cast::<AtomicUsize>());
        counter.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn drop_counter(data: *mut c_void) {
        drop(Box::from_raw(data.cast::<AtomicUsize>()));
    }

    fn counter_ptr(value: usize) -> *mut c_void {
        Box::into_raw(Box::new(AtomicUsize::new(value))).cast()
    }

    #[test]
    fn thread_pool_runs_tasks_and_restarts() {
        unsafe {
            let pool = pdal_thread_pool_create(2, -1);
            assert!(!pool.is_null());

            let first = counter_ptr(0);
            assert!(pdal_thread_pool_add(
                pool,
                first,
                Some(increment_and_drop),
                Some(drop_counter)
            ));
            pdal_thread_pool_await(pool);

            pdal_thread_pool_stop(pool);
            let rejected = counter_ptr(41);
            assert!(!pdal_thread_pool_add(
                pool,
                rejected,
                Some(increment_and_drop),
                Some(drop_counter)
            ));
            drop_counter(rejected);

            pdal_thread_pool_go(pool);
            pdal_thread_pool_resize(pool, 1);
            assert_eq!(pdal_thread_pool_num_threads(pool), 1);

            let second = counter_ptr(0);
            assert!(pdal_thread_pool_add(
                pool,
                second,
                Some(increment_and_drop),
                Some(drop_counter)
            ));
            pdal_thread_pool_await(pool);

            pdal_thread_pool_destroy(pool);
        }
    }

    #[test]
    fn thread_pool_rejects_null_handles_and_callbacks() {
        unsafe {
            assert_eq!(pdal_thread_pool_num_threads(std::ptr::null()), 0);
            pdal_thread_pool_await(std::ptr::null());
            pdal_thread_pool_join(std::ptr::null_mut());

            let pool = pdal_thread_pool_create(1, -1);
            let rejected = counter_ptr(0);
            assert!(!pdal_thread_pool_add(
                pool,
                rejected,
                None,
                Some(drop_counter)
            ));
            drop_counter(rejected);

            pdal_thread_pool_destroy(pool);
        }
    }
}
