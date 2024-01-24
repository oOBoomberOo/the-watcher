use async_trait::async_trait;
use ractor::*;
use crate::model::{Tracker, TrackerId};

mod macros;

pub use macros::*;

pub type Result<T, E = ActorProcessingErr> = ::std::result::Result<T, E>;

pub struct WatcherService;

#[async_trait]
impl Actor for WatcherService {
    type Msg = WatcherMsg;
    type State = WatcherState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(WatcherState::default())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        msg.handle(state).await
    }
}

define_message! {
    pub msg WatcherMsg for WatcherState {
        tick(state) -> Result<()> {
            todo!()
        }

        add(state, tracker: Tracker) -> Result<()> {
            todo!()
        }

        remove(state, id: TrackerId) -> Result<()> {
            todo!()
        }

        update(state, tracker: Tracker) -> Result<()> {
            todo!()
        }
    }
}

#[derive(Default)]
pub struct WatcherState {}
