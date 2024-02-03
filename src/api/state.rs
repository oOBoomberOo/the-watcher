use std::{ops::Deref, sync::Arc};

use derive_new::new;

use crate::{
    database::Database,
    service::{tracker_manager::TrackerManager, youtube::YouTube},
};

#[derive(Debug, Clone, new)]
pub struct App {
    pub manager: Arc<TrackerManager>,
    pub database: Database,
    pub youtube: YouTube,
}

impl App {
    pub fn youtube(&self) -> &YouTube {
        &self.youtube
    }
}

impl Deref for App {
    type Target = TrackerManager;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl<'a> From<&'a App> for &'a Database {
    fn from(app: &'a App) -> Self {
        &app.database
    }
}

pub fn create_app(database: Database, youtube: YouTube) -> App {
    let manager = TrackerManager::new(youtube.clone(), database.clone());

    App {
        manager: Arc::new(manager),
        database,
        youtube,
    }
}
