use tokio::sync::mpsc::{Receiver, Sender};

pub trait Pipe {
    type Sink;

    fn pipe(self, sink: Self::Sink);
}

impl<T> Pipe for Receiver<T>
where
    T: Send + 'static,
{
    type Sink = Sender<T>;

    fn pipe(mut self, sink: Self::Sink) {
        tokio::spawn(async move {
            while let Some(item) = self.recv().await {
                if sink.send(item).await.is_err() {
                    break;
                }
            }
        });
    }
}
