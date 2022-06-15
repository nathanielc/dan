use crate::compiler::{Code, Instruction, Value};

use {
    async_std::task,
    futures::{
        future::{BoxFuture, FutureExt},
        task::{waker_ref, ArcWake},
    },
    std::{
        convert::TryInto,
        fmt,
        sync::mpsc::{sync_channel, Receiver, SyncSender},
        sync::{Arc, Mutex},
        task::{Context, Poll},
        time::Duration,
    },
};

const STACK_SIZE: usize = 512;

pub trait Engine<'a> {
    fn wait(&self, d: Duration) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            task::sleep(d).await;
            None
        })
    }
    fn when(&self, path: &str, value: &str) -> BoxFuture<'a, Option<String>>;
    fn set(&self, path: &str, value: &str) -> BoxFuture<'a, Option<String>>;
    fn get(&self, path: &str) -> BoxFuture<'a, Option<String>>;
}

pub struct VM<'a, E: Engine<'a>> {
    engine: E,
    code: Code,
    // Queue of threads ready to run.
    // A thread is never preempted.
    // Consuming a thread from this queue either
    // runs to completion or it gets blocked.
    //
    // A blocked thread is driven by the executor,
    // once woken the thread is queue back into this channel.
    // Repeat.
    threads: Vec<Thread<'a>>,

    // Send threads into ready queue to be driven
    thread_sender: SyncSender<Arc<Thread<'a>>>,
    ready_queue: Receiver<Arc<Thread<'a>>>,
}

struct Thread<'a> {
    ip: usize,
    stack: [Value; STACK_SIZE],
    stack_ptr: usize, // points to the next free space

    /// In-progress future that should be pushed to completion.
    ///
    /// The `Mutex` is not necessary for correctness, since we only have
    /// one thread executing tasks at once. However, Rust isn't smart
    /// enough to know that `future` is only mutated from one thread,
    /// so we need to use the `Mutex` to prove thread-safety. A production
    /// executor would not need this, and could use `UnsafeCell` instead.
    future: Mutex<Option<BoxFuture<'a, Option<String>>>>,

    /// Handle to place the task itself back onto the task queue.
    sender: SyncSender<Arc<Thread<'a>>>,

    /// The result of the future.
    result: Option<String>,
}

impl<'a> fmt::Debug for Thread<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("ip", &self.ip)
            .field("stack_ptr", &self.stack_ptr)
            .finish()
    }
}
impl<'a> ArcWake for Thread<'a> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Implement `wake` by sending this task back onto the task channel
        // so that it will be polled again by the executor.
        let cloned = arc_self.clone();
        arc_self.sender.send(cloned).expect("too many tasks queued");
    }
}

impl<'a> Thread<'a> {
    fn new(ip: usize, sender: SyncSender<Arc<Thread<'a>>>) -> Thread<'a> {
        Thread {
            ip,
            stack: unsafe { std::mem::zeroed() },
            stack_ptr: 0,
            sender,
            future: Mutex::new(None),
            result: None,
        }
    }
    fn from_spawn(&self, ip: usize) -> Thread<'a> {
        Thread {
            ip,
            stack: self.stack.clone(),
            stack_ptr: self.stack_ptr,
            sender: self.sender.clone(),
            future: Mutex::new(None),
            result: None,
        }
    }
    pub fn pick(&mut self, depth: usize) {
        self.push(self.stack[self.stack_ptr - 1 - depth].clone());
    }

    pub fn push(&mut self, value: Value) {
        self.stack[self.stack_ptr] = value;
        self.stack_ptr += 1; // ignoring the potential stack overflow
    }

    pub fn pop(&mut self) -> Value {
        // ignoring the potential of stack underflow
        // cloning rather than mem::replace for easier testing
        let v = self.stack[self.stack_ptr - 1].clone();
        self.stack_ptr -= 1;
        v
    }
}

impl<'a, E: Engine<'a>> VM<'a, E> {
    pub fn new(code: Code, engine: E) -> VM<'a, E> {
        // Maximum number of tasks to allow queueing in the channel at once.
        // This is just to make `sync_channel` happy, and wouldn't be present in
        // a real executor.
        const MAX_QUEUED_TASKS: usize = 10_000;
        let (thread_sender, ready_queue) = sync_channel(MAX_QUEUED_TASKS);
        // Create the _main_ thread
        let thread = Thread::new(0, thread_sender.clone());
        let mut threads = Vec::new();
        threads.push(thread);
        VM {
            engine,
            code,
            threads,
            thread_sender,
            ready_queue,
        }
    }
    pub fn run(&mut self) {
        let mut active_thread_count = 1;
        // Drive all threads to completion
        while active_thread_count > 0 {
            let mut new_threads = Vec::new();
            for mut thread in self.threads.drain(..) {
                loop {
                    let inst_addr = thread.ip;
                    thread.ip += 1;

                    log::debug!("inst: {:?}", self.code.instructions[inst_addr]);
                    match self.code.instructions[inst_addr] {
                        Instruction::Constant(const_idx) => {
                            thread.push(self.code.constants[const_idx as usize].clone());
                        }
                        Instruction::Print => {
                            println!("{}", thread.pop());
                        }
                        Instruction::Pick(depth) => {
                            thread.pick(depth);
                        }
                        Instruction::Pop => {
                            thread.pop();
                        }
                        Instruction::Spawn(ip) => {
                            active_thread_count += 1;
                            new_threads.push(thread.from_spawn(thread.ip));
                            // update local ip to jump location
                            thread.ip = ip;
                        }
                        Instruction::Term => {
                            // This thread is complete.
                            // The thread will be dropped and forgotten
                            active_thread_count -= 1;
                            break;
                        }
                        Instruction::When => {
                            let value: String = thread.pop().try_into().unwrap();
                            let path: String = thread.pop().try_into().unwrap();
                            // Creature future and queue it for the executor
                            let future = self.engine.when(path.as_str(), value.as_str());
                            let future = future.boxed();
                            {
                                let mut future_slot = thread.future.lock().unwrap();
                                *future_slot = Some(future.boxed());
                            }
                            self.thread_sender
                                .send(Arc::new(thread))
                                .expect("too many tasks queued");
                            break;
                        }
                        Instruction::Set => {
                            let value: String = thread.pop().try_into().unwrap();
                            let path: String = thread.pop().try_into().unwrap();
                            // Creature future and queue it for the executor
                            let future = self.engine.set(path.as_str(), value.as_str());
                            let future = future.boxed();
                            {
                                let mut future_slot = thread.future.lock().unwrap();
                                *future_slot = Some(future.boxed());
                            }
                            self.thread_sender
                                .send(Arc::new(thread))
                                .expect("too many tasks queued");
                            break;
                        }
                        Instruction::Get => {
                            let path: String = thread.pop().try_into().unwrap();
                            // Creature future and queue it for the executor
                            let future = self.engine.get(path.as_str());
                            let future = future.boxed();
                            {
                                let mut future_slot = thread.future.lock().unwrap();
                                *future_slot = Some(future.boxed());
                            }
                            self.thread_sender
                                .send(Arc::new(thread))
                                .expect("too many tasks queued");
                            break;
                        }
                        Instruction::GetResult => {
                            if let Some(result) = thread.result.clone() {
                                thread.push(Value::Str(result));
                            } else {
                                panic!("no result from get")
                            }
                        }
                        Instruction::Wait => {
                            let v = thread.pop();
                            match v {
                                Value::Duration(d) => {
                                    // Creature future and queue it for the executor
                                    let future = self.engine.wait(d);
                                    {
                                        let mut future_slot = thread.future.lock().unwrap();
                                        *future_slot = Some(future.boxed());
                                    }
                                    self.thread_sender
                                        .send(Arc::new(thread))
                                        .expect("too many tasks queued");
                                    break;
                                }
                                _ => {
                                    panic!("wait arg must be a duration")
                                }
                            };
                        }
                    }
                }
            }
            if !new_threads.is_empty() {
                self.threads.append(&mut new_threads);
                // Eagerly process new threads
                continue;
            }
            if active_thread_count == 0 {
                // All threads have completed no need to
                // wait for threads to wake up.
                break;
            }

            // Wait for any blocked threads to wake up
            while let Ok(thread) = self.ready_queue.recv() {
                // Take the future, and if it has not yet completed (is still Some),
                // poll it in an attempt to complete it.
                let mut completed = false;
                let mut result: Option<String> = None;
                {
                    let mut future_slot = thread.future.lock().unwrap();
                    if let Some(mut future) = future_slot.take() {
                        // Create a `LocalWaker` from the task itself
                        let waker = waker_ref(&thread);
                        let context = &mut Context::from_waker(&*waker);
                        // `BoxFuture<T>` is a type alias for
                        // `Pin<Box<dyn Future<Output = T> + Send + 'static>>`.
                        // We can get a `Pin<&mut dyn Future + Send + 'static>`
                        // from it by calling the `Pin::as_mut` method.
                        match future.as_mut().poll(context) {
                            Poll::Pending => {
                                // We're not done processing the future, so put it
                                // back in its task to be run again in the future.
                                *future_slot = Some(future);
                            }
                            Poll::Ready(r) => {
                                // Queue thread to run again
                                completed = true;

                                result = r;
                            }
                        }
                    }
                }
                if completed {
                    let mut t =
                        Arc::try_unwrap(thread).expect("thread still has another reference");
                    t.result = result;
                    self.threads.push(t);
                    // Eagerly process ready thread
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use async_std::future;
    use futures::future::BoxFuture;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::compiler::Interpreter;
    use crate::Compile;

    struct TestEngine {
        wait_count: AtomicUsize,
        wait_args: Mutex<Vec<Duration>>,
        when_count: AtomicUsize,
        when_args: Mutex<Vec<(String, String)>>,
        set_count: AtomicUsize,
        set_args: Mutex<Vec<(String, String)>>,
        get_count: AtomicUsize,
        get_args: Mutex<Vec<String>>,
    }
    impl TestEngine {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                wait_count: AtomicUsize::new(0),
                wait_args: Mutex::new(Vec::new()),
                when_count: AtomicUsize::new(0),
                when_args: Mutex::new(Vec::new()),
                set_count: AtomicUsize::new(0),
                set_args: Mutex::new(Vec::new()),
                get_count: AtomicUsize::new(0),
                get_args: Mutex::new(Vec::new()),
            })
        }
    }

    impl<'a> Engine<'a> for Arc<TestEngine> {
        fn wait(&self, d: Duration) -> BoxFuture<'a, Option<String>> {
            self.wait_count.fetch_add(1, Ordering::SeqCst);
            self.wait_args.lock().unwrap().push(d.clone());
            Box::pin(future::ready(None))
        }

        fn when(&self, path: &str, value: &str) -> BoxFuture<'a, Option<String>> {
            self.when_count.fetch_add(1, Ordering::SeqCst);
            self.when_args
                .lock()
                .unwrap()
                .push((path.to_string(), value.to_string()));
            Box::pin(future::ready(None))
        }

        fn set(&self, path: &str, value: &str) -> BoxFuture<'a, Option<String>> {
            self.set_count.fetch_add(1, Ordering::SeqCst);
            self.set_args
                .lock()
                .unwrap()
                .push((path.to_string(), value.to_string()));
            Box::pin(future::ready(None))
        }

        fn get(&self, path: &str) -> BoxFuture<'a, Option<String>> {
            self.get_count.fetch_add(1, Ordering::SeqCst);
            self.get_args.lock().unwrap().push(path.to_string());
            Box::pin(future::ready(Some("get value".to_string())))
        }
    }
    #[test]
    fn test_when() {
        let source = "
        when path is \"on\" print \"off\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let mut vm = VM::new(code, te.clone());
        vm.run();

        assert_eq!(0, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));
        assert_eq!(0, te.get_count.load(Ordering::SeqCst));

        assert_eq!(1, te.when_count.load(Ordering::SeqCst));
        assert_eq!(
            vec![("path".to_string(), "on".to_string())],
            te.when_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<(String, String)>>(),
        );
    }
    #[test]
    fn test_wait() {
        let source = "
        wait 1s print \"done\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let mut vm = VM::new(code, te.clone());
        vm.run();

        assert_eq!(0, te.when_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));
        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        assert_eq!(1, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(
            vec![Duration::from_secs(1),],
            te.wait_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<Duration>>(),
        );
    }
    #[test]
    fn test_set() {
        let source = "
        set path/to/value \"on\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let mut vm = VM::new(code, te.clone());
        vm.run();

        assert_eq!(0, te.when_count.load(Ordering::SeqCst));
        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        assert_eq!(0, te.wait_count.load(Ordering::SeqCst));

        assert_eq!(1, te.set_count.load(Ordering::SeqCst));
        assert_eq!(
            vec![("path/to/value".to_string(), "on".to_string())],
            te.set_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<(String, String)>>(),
        );
    }
    #[test]
    fn test_get() {
        let source = "
        get path/to/value
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let mut vm = VM::new(code, te.clone());
        vm.run();

        assert_eq!(0, te.when_count.load(Ordering::SeqCst));
        assert_eq!(0, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));

        assert_eq!(1, te.get_count.load(Ordering::SeqCst));
        assert_eq!(
            vec!["path/to/value".to_string()],
            te.get_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<String>>(),
        );
    }
    #[test]
    fn test_many_threads() {
        let source = "
        wait 5s print \"a\"
        wait 4s print \"b\"
        wait 3s print \"c\"
        wait 2s print \"d\"
        wait 1s print \"e\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let mut vm = VM::new(code, te.clone());
        vm.run();

        assert_eq!(0, te.when_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));
        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        assert_eq!(5, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(
            vec![
                Duration::from_secs(5),
                Duration::from_secs(4),
                Duration::from_secs(3),
                Duration::from_secs(2),
                Duration::from_secs(1),
            ],
            te.wait_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<Duration>>(),
        );
    }
}
