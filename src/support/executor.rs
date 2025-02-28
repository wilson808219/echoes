use tokio::spawn;

#[derive(Clone)]
pub(crate) struct Executor;

impl<F> hyper::rt::Executor<F> for Executor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        spawn(fut);
    }
}

impl Executor {
    pub fn new() -> Self {
        Executor
    }
}
