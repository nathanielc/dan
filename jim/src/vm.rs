use {
    anyhow::Result,
    async_trait::async_trait,
    futures::future::{BoxFuture, FutureExt},
    std::{convert::TryInto, fmt, sync::Arc, time::Duration},
    tokio::task::JoinHandle,
    tokio::time,
};

use tokio::sync::mpsc::{self, Sender};

use crate::compiler::{Code, Instruction, Value};

const STACK_SIZE: usize = 512;

#[async_trait]
pub trait Engine: Clone + Send + Sync {
    async fn wait(&self, d: Duration) -> Result<()> {
        time::sleep(d).await;
        Ok(())
    }
    async fn when(&self, path: &str, value: &str) -> Result<()>;
    async fn set(&self, path: &str, value: &str) -> Result<()>;
    async fn get(&self, path: &str) -> Result<String>;
}

struct Thread<E: Engine> {
    engine: E,
    code: Arc<Code>,
    ip: usize,
    stack: [Value; STACK_SIZE],
    stack_ptr: usize, // points to the next free space
    sender: Sender<JoinHandle<Result<()>>>,
}

impl<E: Engine> fmt::Debug for Thread<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("ip", &self.ip)
            .field("stack_ptr", &self.stack_ptr)
            .finish()
    }
}

impl<E: Engine + 'static> Thread<E> {
    fn new(
        engine: E,
        code: Arc<Code>,
        ip: usize,
        sender: Sender<JoinHandle<Result<()>>>,
    ) -> Thread<E> {
        Thread {
            engine,
            code,
            ip,
            stack: unsafe { std::mem::zeroed() },
            stack_ptr: 0,
            sender,
        }
    }
    fn from_spawn(&self, ip: usize) -> Thread<E> {
        Thread {
            engine: self.engine.clone(),
            code: self.code.clone(),
            ip,
            stack: self.stack.clone(),
            stack_ptr: self.stack_ptr,
            sender: self.sender.clone(),
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
    fn run(self) -> BoxFuture<'static, Result<()>> {
        // Use boxed indirection to avoid recusive async calls.
        // See https://rust-lang.github.io/async-book/07_workarounds/04_recursion.html
        self._run().boxed()
    }
    async fn _run(mut self) -> Result<()> {
        loop {
            let inst_addr = self.ip;
            self.ip += 1;

            log::debug!("inst: {:?}", self.code.instructions[inst_addr]);
            match self.code.instructions[inst_addr] {
                Instruction::Constant(const_idx) => {
                    self.push(self.code.constants[const_idx as usize].clone());
                }
                Instruction::Print => {
                    println!("{}", self.pop());
                }
                Instruction::Pick(depth) => {
                    self.pick(depth);
                }
                Instruction::Pop => {
                    self.pop();
                }
                Instruction::Spawn(ip) => {
                    let new_thread = self.from_spawn(self.ip);
                    let join_handle = tokio::spawn(new_thread.run());
                    // Track every spawned thread, so we can join on them
                    self.sender.send(join_handle).await?;

                    // update local ip to jump location
                    self.ip = ip;
                }
                Instruction::Term => {
                    // This thread is complete.
                    // The thread will be dropped and forgotten
                    return Ok(());
                }
                Instruction::When => {
                    let value: String = self.pop().try_into()?;
                    let path: String = self.pop().try_into()?;
                    // Creature future and queue it for the executor
                    self.engine.when(path.as_str(), value.as_str()).await?;
                }
                Instruction::Set => {
                    let value: String = self.pop().try_into()?;
                    let path: String = self.pop().try_into()?;
                    // Creature future and queue it for the executor
                    self.engine.set(path.as_str(), value.as_str()).await?;
                }
                Instruction::Get => {
                    let path: String = self.pop().try_into()?;
                    // Creature future and queue it for the executor
                    let result = self.engine.get(path.as_str()).await?;
                    self.push(Value::Str(result));
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
            }
        }
    }
}

pub struct VM<E: Engine> {
    engine: E,
}
impl<E: Engine + 'static> VM<E> {
    pub fn new(engine: E) -> VM<E> {
        VM { engine }
    }
    pub async fn run(&self, code: Code) -> Result<()> {
        // Create channel for thread counts

        let (thread_join_send, mut thread_join_recv) = mpsc::channel(100);

        // Create and spawn main thread
        let thread = Thread::new(self.engine.clone(), Arc::new(code), 0, thread_join_send);
        thread.run().await?;

        // Wait until all threads are completed before returning
        loop {
            if let Some(thread_join) = thread_join_recv.recv().await {
                thread_join.await??;
            } else {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use async_std::future;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

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

    #[async_trait]
    impl Engine for Arc<TestEngine> {
        async fn wait(&self, d: Duration) -> Result<()> {
            self.wait_count.fetch_add(1, Ordering::SeqCst);
            self.wait_args.lock().unwrap().push(d.clone());
            future::ready(Ok(())).await
        }

        async fn when(&self, path: &str, value: &str) -> Result<()> {
            self.when_count.fetch_add(1, Ordering::SeqCst);
            self.when_args
                .lock()
                .unwrap()
                .push((path.to_string(), value.to_string()));
            future::ready(Ok(())).await
        }

        async fn set(&self, path: &str, value: &str) -> Result<()> {
            self.set_count.fetch_add(1, Ordering::SeqCst);
            self.set_args
                .lock()
                .unwrap()
                .push((path.to_string(), value.to_string()));
            future::ready(Ok(())).await
        }

        async fn get(&self, path: &str) -> Result<String> {
            self.get_count.fetch_add(1, Ordering::SeqCst);
            self.get_args.lock().unwrap().push(path.to_string());
            future::ready(Ok("get value".to_string())).await
        }
    }
    #[tokio::test]
    async fn test_when() {
        let source = "
        when path is \"on\" print \"off\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let vm = VM::new(te.clone());
        vm.run(code).await.unwrap();

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
    #[tokio::test]
    async fn test_wait() {
        let source = "
        wait 1s print \"done\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let vm = VM::new(te.clone());
        vm.run(code).await.unwrap();

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
    #[tokio::test]
    async fn test_set() {
        let source = "
        set path/to/value \"on\"
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let vm = VM::new(te.clone());
        vm.run(code).await.unwrap();

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
    #[tokio::test]
    async fn test_get() {
        let source = "
        get path/to/value
";
        let code = Interpreter::from_source(source);
        log::debug!("code:     {:?}", code);

        let te = TestEngine::new();
        let vm = VM::new(te.clone());
        vm.run(code).await.unwrap();

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
    #[tokio::test]
    async fn test_many_threads() {
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
        let vm = VM::new(te.clone());
        vm.run(code).await.unwrap();

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