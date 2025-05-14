use is_type::Is;
use std::future::Future;
use tokio::sync::mpsc;

pub struct Robot<E> {
    count: usize,
    tx: mpsc::UnboundedSender<Result<(), E>>,
    rx: mpsc::UnboundedReceiver<Result<(), E>>,
}

impl<E: Send + 'static> Robot<E> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Robot { count: 0, tx, rx }
    }

    pub fn spawn<T>(&mut self, task: T)
    where
        T: Future + Send + 'static,
        T::Output: Is<Type = Result<(), E>>,
    {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let res = task.await;
            match tx.send(res.into_val()) {
                Ok(()) => (),
                Err(_) => panic!("Tidak mungkin terjadi! tx.send gagal"),
            }
        });

        self.count += 1;
    }

    pub async fn run(mut self) -> Result<(), E> {
        std::mem::drop(self.tx);
        let mut i = 0;
        loop {
            match self.rx.recv().await {
                None => {
                    assert_eq!(i, self.count);
                    break Ok(());
                }
                Some(Ok(())) => {
                    assert!(i < self.count);
                }
                Some(Err(e)) => {
                    assert!(i < self.count);
                    return Err(e);
                }
            }
            i += 1;
        }
    }
}
