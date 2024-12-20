use std::sync::atomic::AtomicUsize;

use {
    anyhow::Result,
    async_trait::async_trait,
    chrono::{DateTime, Local},
    futures::future::{BoxFuture, FutureExt},
    std::{convert::TryInto, fmt, sync::Arc, time::Duration},
    tokio::{
        io::AsyncWriteExt,
        select,
        sync::{
            broadcast,
            mpsc::{self, Sender},
        },
        task::JoinHandle,
        time,
    },
};

use tokio::io;

use crate::compiler::{Code, Instruction, TimeOfDay, Value};

const STACK_SIZE: usize = 512;

#[async_trait]
pub trait Engine: Clone + Send + Sync {
    async fn print(&self, msg: &str) -> Result<()> {
        let mut stdout = io::stdout();
        stdout.write_all(msg.as_bytes()).await?;
        stdout.write_all("\n".as_bytes()).await?;
        stdout.flush().await?;
        Ok(())
    }
    async fn wait(&self, d: Duration) -> Result<()> {
        time::sleep(d).await;
        Ok(())
    }
    async fn get(&self, path: &str) -> Result<Vec<u8>>;
    async fn set(&self, path: &str, value: Vec<u8>) -> Result<()>;
}

struct Thread<E: Engine> {
    cancel_rx: broadcast::Receiver<()>,
    ctx: ThreadContext<E>,
}
struct ThreadContext<E: Engine> {
    id: usize,
    engine: E,
    code: Arc<Code>,
    ip: usize,
    stack: [Value; STACK_SIZE],
    stack_ptr: usize, // points to the next free space
    call_stack: Vec<usize>,
    sender: Sender<JoinHandle<Result<()>>>,
    cancel_tx: broadcast::Sender<()>,
}

impl<E: Engine> fmt::Debug for Thread<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("ip", &self.ctx.ip)
            .field("stack_ptr", &self.ctx.stack_ptr)
            .finish()
    }
}

enum StepResult {
    Continue,
    SceneContext,
    Break,
}

impl<E: Engine + 'static> Thread<E> {
    fn new(
        engine: E,
        code: Arc<Code>,
        ip: usize,
        sender: Sender<JoinHandle<Result<()>>>,
    ) -> Thread<E> {
        let (cancel_tx, cancel_rx) = broadcast::channel(1);
        Thread {
            cancel_rx,
            ctx: ThreadContext {
                id: Thread::<E>::next_id(),
                engine,
                code,
                ip,
                stack: unsafe { std::mem::zeroed() },
                stack_ptr: 0,
                call_stack: Vec::new(),
                sender,
                cancel_tx,
            },
        }
    }

    fn next_id() -> usize {
        static ID: AtomicUsize = AtomicUsize::new(0);
        ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
    fn run(self, shutdown: broadcast::Receiver<()>) -> BoxFuture<'static, Result<()>> {
        // Use boxed indirection to avoid recusive async calls.
        // See https://rust-lang.github.io/async-book/07_workarounds/04_recursion.html
        self._run(shutdown).boxed()
    }
    async fn _run(mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        loop {
            select! {
                // TODO: Restructure so that we do not have to pre-emptively resubsribe for each
                // step
                step = self.ctx.step(shutdown.resubscribe()) => {
                    match step? {
                        StepResult::Continue => {}
                        StepResult::SceneContext => {
                            let (cancel_tx, cancel_rx) = broadcast::channel(1);
                            self.cancel_rx = cancel_rx;
                            self.ctx.cancel_tx = cancel_tx;
                        },
                        StepResult::Break => break,
                    }
                },
                _ = shutdown.recv() => break,
                _ = self.cancel_rx.recv() => break,
            }
        }
        Ok(())
    }
}
impl<E: Engine + 'static> ThreadContext<E> {
    fn spawn(&self, ip: usize) -> Thread<E> {
        let cancel_tx = self.cancel_tx.clone();
        let cancel_rx = self.cancel_tx.subscribe();
        Thread {
            ctx: ThreadContext {
                id: Thread::<E>::next_id(),
                engine: self.engine.clone(),
                code: self.code.clone(),
                ip,
                stack: self.stack.clone(),
                stack_ptr: self.stack_ptr,
                call_stack: Vec::new(),
                sender: self.sender.clone(),
                cancel_tx,
            },
            cancel_rx,
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

    async fn step(&mut self, shutdown: broadcast::Receiver<()>) -> Result<StepResult> {
        let inst_addr = self.ip;
        self.ip += 1;

        log::debug!("inst[{}]: {:?}", self.id, self.code.instructions[inst_addr]);
        match self.code.instructions[inst_addr] {
            Instruction::Constant(const_idx) => {
                self.push(self.code.constants[const_idx].clone());
            }
            Instruction::Print => {
                let msg = format!("{}", self.pop());
                self.engine.print(msg.as_str()).await?;
            }
            Instruction::Pick(depth) => {
                self.pick(depth);
            }
            Instruction::Pop => {
                self.pop();
            }
            Instruction::Swap => {
                let a = self.pop();
                let b = self.pop();
                self.push(a);
                self.push(b);
            }
            Instruction::Spawn(ip) => {
                let new_thread = self.spawn(self.ip);
                let join_handle = tokio::spawn(new_thread.run(shutdown));
                // Track every spawned thread, so we can join on them
                self.sender.send(join_handle).await?;

                // update local ip to jump location
                self.ip = ip;
            }
            Instruction::Jump(ip) => {
                self.ip = ip;
            }
            Instruction::Term => {
                // This thread is complete.
                // The thread will be dropped and forgotten
                return Ok(StepResult::Break);
            }
            Instruction::Get => {
                let path: String = self.pop().try_into()?;
                // Creature future and queue it for the executor
                let value = self.engine.get(path.as_str()).await?;
                self.push(value[..].try_into()?);
            }
            Instruction::Set => {
                let value: Vec<u8> = self.pop().try_into()?;
                let path: String = self.pop().try_into()?;
                // Creature future and queue it for the executor
                self.engine.set(path.as_str(), value).await?;
            }
            Instruction::Wait => {
                let v = self.pop();
                match v {
                    Value::Duration(d) => {
                        self.engine.wait(d).await?;
                    }
                    _ => {
                        panic!("wait arg must be a duration")
                    }
                };
            }
            Instruction::Call => {
                self.call_stack.push(self.ip);
                self.ip = match self.pop() {
                    Value::Jump(ip) => ip,
                    _ => panic!("call pointer not a jump value"),
                };
            }
            Instruction::Return => {
                self.ip = self.call_stack.pop().unwrap();
            }
            Instruction::SceneContext => {
                return Ok(StepResult::SceneContext);
            }
            Instruction::Stop => {
                let count = self.cancel_tx.send(()).unwrap();
                log::debug!("stopped {} scene threads", count);
            }
            Instruction::At => {
                let v = self.pop();
                match v {
                    Value::Time(t) => {
                        let then: DateTime<Local> = match t {
                            TimeOfDay::Sunrise => todo!(),
                            TimeOfDay::Sunset => todo!(),
                            TimeOfDay::HM(h, m) => Local::today().and_hms(h, m, 0),
                        };
                        let now: DateTime<Local> = Local::now();
                        let mut diff = then.timestamp() - now.timestamp();
                        if diff <= 0 {
                            // If the time has passed today wait for the next one.
                            diff += 24 * 60 * 60;
                        }
                        let d = Duration::from_secs(diff as u64);
                        self.engine.wait(d).await?;
                    }
                    _ => {
                        panic!("at arg must be a time")
                    }
                };
            }
            Instruction::Equal => {
                let rhs = self.pop();
                let lhs = self.pop();
                self.push(Value::Bool(rhs == lhs))
            }
            Instruction::JmpNot(ip) => {
                let v = self.pop();
                match v {
                    Value::Bool(true) => {
                        // Do not jump
                    }
                    Value::Bool(false) => {
                        self.ip = ip;
                    }
                    _ => {
                        panic!("value must be a bool")
                    }
                }
            }
            Instruction::Index => {
                if let Value::Str(prop) = self.pop() {
                    if let Value::Object(props) = self.pop() {
                        if let Some(v) = props.get(&prop) {
                            self.push(v.to_owned());
                        } else {
                            panic!("object does not have property")
                        }
                    } else {
                        panic!("cannot index into non object values")
                    }
                } else {
                    panic!("index property must be a string value")
                }
            }
        };
        Ok(StepResult::Continue)
    }
}

pub struct VM<E: Engine> {
    engine: E,
}
impl<E: Engine + 'static> VM<E> {
    pub fn new(engine: E) -> VM<E> {
        VM { engine }
    }
    pub async fn run(&self, code: Code, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        // Create channel for thread join handles
        let (thread_join_send, mut thread_join_recv) = mpsc::channel(100);

        // Create and run main thread
        let thread = Thread::new(self.engine.clone(), Arc::new(code), 0, thread_join_send);
        thread.run(shutdown.resubscribe()).await?;

        // Now that the main thread is completed wait until all other threads
        // are completed before returning.
        //
        // NOTE: The thread_join_send, will be dropped once all active threads are
        // completed and this loop will terminate.
        loop {
            select! {
                thread_join = thread_join_recv.recv() => {
                    if let Some(thread_join) = thread_join {
                        select! {
                        _ = thread_join => {},
                        _ = shutdown.recv() => break,
                        };
                    } else {
                        // All threads have completed
                        break;
                    }
                }
                _ = shutdown.recv() => break,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use async_std::future;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        task::Poll,
    };

    use super::*;
    use crate::compiler::Interpreter;
    use crate::Compile;

    struct TestEngine {
        print_count: AtomicUsize,
        print_args: Mutex<Vec<String>>,
        wait_count: AtomicUsize,
        wait_args: Mutex<Vec<Duration>>,
        get_count: AtomicUsize,
        get_args: Mutex<Vec<String>>,
        set_count: AtomicUsize,
        set_args: Mutex<Vec<(String, String)>>,
    }
    impl TestEngine {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                print_count: AtomicUsize::new(0),
                print_args: Mutex::new(Vec::new()),
                wait_count: AtomicUsize::new(0),
                wait_args: Mutex::new(Vec::new()),
                get_count: AtomicUsize::new(0),
                get_args: Mutex::new(Vec::new()),
                set_count: AtomicUsize::new(0),
                set_args: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl Engine for Arc<TestEngine> {
        async fn print(&self, msg: &str) -> Result<()> {
            self.print_count.fetch_add(1, Ordering::SeqCst);
            self.print_args.lock().unwrap().push(msg.to_string());
            future::ready(Ok(())).await
        }
        async fn wait(&self, d: Duration) -> Result<()> {
            self.wait_count.fetch_add(1, Ordering::SeqCst);
            self.wait_args.lock().unwrap().push(d);
            future::ready(Ok(())).await
        }

        async fn get(&self, path: &str) -> Result<Vec<u8>> {
            let count = self.get_count.fetch_add(1, Ordering::SeqCst);
            self.get_args.lock().unwrap().push(path.to_string());
            println!("count {}", count);
            if count == 0 {
                future::ready(Ok("true".as_bytes().to_vec())).await
            } else {
                empty().await
            }
        }

        async fn set(&self, path: &str, value: Vec<u8>) -> Result<()> {
            self.set_count.fetch_add(1, Ordering::SeqCst);
            self.set_args
                .lock()
                .unwrap()
                .push((path.to_string(), String::from_utf8(value.into()).unwrap()));
            future::ready(Ok(())).await
        }
    }

    use core::marker;
    use futures::task;
    use futures::Future;

    /// A future which is never resolved.
    ///
    /// This future can be created with the `empty` function.
    #[derive(Debug)]
    #[must_use = "futures do nothing unless polled"]
    pub struct Empty<T> {
        _data: marker::PhantomData<T>,
    }

    /// Creates a future which never resolves, representing a computation that never
    /// finishes.
    ///
    /// The returned future will forever return `Async::Pending`.
    pub fn empty<T>() -> Empty<T> {
        Empty {
            _data: marker::PhantomData,
        }
    }

    impl<T> Future for Empty<T> {
        type Output = T;

        fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut task::Context<'_>) -> Poll<Self::Output> {
            Poll::Pending
        }
    }

    fn run_vm(source: &str) -> (Arc<TestEngine>, broadcast::Sender<()>) {
        let code = Interpreter::from_source(source).unwrap();
        let te = TestEngine::new();
        let vm = VM::new(te.clone());
        let (shutdown_tx, shutdown_rx) = broadcast::channel(2);
        tokio::spawn(async move {
            vm.run(code, shutdown_rx).await.unwrap();
        });
        (te, shutdown_tx)
    }
    #[tokio::test]
    async fn test_print() {
        let source = "
        print 1;
";

        let (te, shutdown) = run_vm(source);
        // TODO: remove this sleep
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(1, te.print_count.load(Ordering::SeqCst));
        assert_eq!(
            vec!["1".to_string(),],
            te.print_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<String>>(),
        );
        assert_eq!(0, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));

        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        let _ = shutdown.send(());
    }
    #[tokio::test]
    async fn test_as() {
        let source = "
        print 1 as x in x;
";

        let (te, shutdown) = run_vm(source);
        // TODO: remove this sleep
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(1, te.print_count.load(Ordering::SeqCst));
        assert_eq!(
            vec!["1".to_string(),],
            te.print_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<String>>(),
        );
        assert_eq!(0, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));

        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        let _ = shutdown.send(());
    }
    #[tokio::test]
    async fn test_index() {
        let source = "
        let o = {x: 1};
        print o.x;
";

        let (te, shutdown) = run_vm(source);
        // TODO: remove this sleep
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(1, te.print_count.load(Ordering::SeqCst));
        assert_eq!(
            vec!["1".to_string(),],
            te.print_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<String>>(),
        );
        assert_eq!(0, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));

        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        let _ = shutdown.send(());
    }

    #[tokio::test]
    async fn test_when() {
        let source = "
        when <path> print \"off\";
";

        let (te, shutdown) = run_vm(source);
        // TODO: remove this sleep
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(1, te.print_count.load(Ordering::SeqCst));
        assert_eq!(
            vec!["off".to_string(),],
            te.print_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<String>>(),
        );
        assert_eq!(0, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));

        assert_eq!(2, te.get_count.load(Ordering::SeqCst));
        assert_eq!(
            vec![("path".to_string()), ("path".to_string()),],
            te.get_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<String>>(),
        );
        let _ = shutdown.send(());
    }
    #[tokio::test]
    async fn test_wait() {
        let source = "
            wait 1s print \"done\";
    ";
        let (te, shutdown) = run_vm(source);
        // TODO: remove this sleep
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));
        assert_eq!(1, te.wait_count.load(Ordering::SeqCst));
        assert_eq!(
            vec![Duration::from_secs(1),],
            te.wait_args
                .lock()
                .unwrap()
                .drain(..)
                .collect::<Vec<Duration>>(),
        );
        let _ = shutdown.send(());
    }
    #[tokio::test]
    async fn test_set() {
        let source = "
            set [path/to/value] \"on\";
    ";
        let (te, shutdown) = run_vm(source);
        // TODO: remove this sleep
        time::sleep(Duration::from_millis(100)).await;

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
        let _ = shutdown.send(());
    }
    #[tokio::test]
    async fn test_many_threads() {
        let source = "
            wait 5s print \"a\";
            wait 4s print \"b\";
            wait 3s print \"c\";
            wait 2s print \"d\";
            wait 1s print \"e\";
    ";
        let (te, shutdown) = run_vm(source);
        // TODO: remove this sleep
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));
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
        let _ = shutdown.send(());
    }
    #[tokio::test]
    async fn test_scene() {
        let source = "
        scene night { print \"x\"; };
        start night;
        stop night;
    ";
        let (te, shutdown) = run_vm(source);
        // TODO: remove this sleep
        time::sleep(Duration::from_millis(100)).await;

        assert_eq!(0, te.get_count.load(Ordering::SeqCst));
        assert_eq!(0, te.set_count.load(Ordering::SeqCst));
        assert_eq!(0, te.wait_count.load(Ordering::SeqCst));
        let _ = shutdown.send(());
    }
}
