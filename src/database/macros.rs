#[macro_export]
macro_rules! define_crud {
    ($model:ty) => {
        use $crate::database::{DatabaseQueryError, IntoDatabase};

        impl $model {
            pub async fn list(db: impl IntoDatabase) -> Result<Vec<Self>, DatabaseQueryError> {
                db.into_database()
                    .select(Self::table())
                    .await
                    .map_err(Into::into)
            }

            pub async fn find(
                id: impl surrealdb::opt::IntoResource<Option<Self>>,
                db: impl IntoDatabase,
            ) -> Result<Option<Self>, DatabaseQueryError> {
                db.into_database().select(id).await.map_err(Into::into)
            }

            pub async fn create(
                &self,
                db: impl IntoDatabase,
            ) -> Result<Vec<Self>, DatabaseQueryError> {
                db.into_database()
                    .create(Self::table())
                    .content(self)
                    .await
                    .map_err(Into::into)
            }

            pub async fn update(
                &self,
                db: impl IntoDatabase,
            ) -> Result<Option<Self>, DatabaseQueryError> {
                db.into_database()
                    .update(self.id())
                    .merge(self)
                    .await
                    .map_err(Into::into)
            }

            pub async fn delete(
                &self,
                db: impl IntoDatabase,
            ) -> Result<Option<Self>, DatabaseQueryError> {
                db.into_database()
                    .delete(self.id())
                    .await
                    .map_err(Into::into)
            }
        }
    };
}

#[macro_export]
macro_rules! define_table {
    ($table:literal: $model:ty = $id:ident) => {
        impl $crate::database::Table for $model {
            fn id(&self) -> &$crate::prelude::Thing {
                self.$id.as_ref()
            }

            fn table() -> &'static str {
                $table
            }
        }
    };
}

/// Defines a method to query the database using SQL.
///
/// # Syntax
/// ```
/// [Base Type] > method_name(...arguments) > [Output Type] where "sql query"
/// ```
/// Where the `Base Type` is the type that the method is being defined for and the `Output Type` is the type that the method will return.
///
/// # Example
///
/// ```rust
/// define_relation! {
///     Tracker > stats(id: TrackerId) > Stats
///         where "SELECT * FROM stats WHERE tracker_id = $id ORDER BY created_at DESC"
/// }
///
/// let stats = Tracker::stats(tracker_id, &db).await?;
/// ```
#[macro_export]
macro_rules! define_relation {
    ($model:ty > $relation:ident ($($binding:ident : $binding_type:ty),*) > $export:ty where $query:literal) => {
        impl $model {
            pub async fn $relation($($binding : $binding_type ,)* db: impl $crate::database::IntoDatabase) -> Result<$export, $crate::database::DatabaseQueryError> {
                use $crate::database::query::Sql;

                db.sql($query)
                    $(.bind((stringify!($binding), $binding)))*
                    .fetch_first()
                    .await
            }
        }
    };
}
