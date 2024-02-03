use crate::database::Id;

pub fn table<T: Id>() -> &'static str {
    T::table()
}

pub fn id<T: Id>(t: &T) -> &crate::database::Thing {
    T::id(t)
}

#[macro_export]
macro_rules! define_model {
    ($model:ty) => {
        impl $model {
            pub async fn list(db: impl Into<&Database>) -> $crate::database::Result<Vec<Self>> {
                db.into()
                    .select($crate::macros::table::<Self>())
                    .await
                    .context(DatabaseQuerySnafu)
            }

            pub async fn find(
                id: impl surrealdb::opt::IntoResource<Option<Self>>, db: impl Into<&Database>,
            ) -> $crate::database::Result<Option<Self>> {
                db.into().select(id).await.context(DatabaseQuerySnafu)
            }

            pub async fn create(&self, db: impl Into<&Database>) -> $crate::database::Result<Vec<Self>> {
                db.into().create($crate::macros::table::<Self>())
                    .content(self)
                    .await
                    .context(DatabaseQuerySnafu)
            }

            pub async fn update(&self, db: impl Into<&Database>) -> $crate::database::Result<Option<Self>> {
                db.into().update($crate::macros::id(self))
                    .merge(self)
                    .await
                    .context(DatabaseQuerySnafu)
            }

            pub async fn delete(&self, db: impl Into<&Database>) -> $crate::database::Result<Option<Self>> {
                db.into().delete($crate::macros::id(self))
                    .await
                    .context(DatabaseQuerySnafu)
            }
        }
    };
}

#[macro_export]
macro_rules! define_id {
    ($table:literal, $model:ty : $self:ident => $getter:expr) => {
        impl $crate::database::Id for $model {
            fn id(&$self) -> &$crate::database::Thing {
                $getter
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
            pub async fn $relation($($binding : $binding_type ,)* db: impl Into<&Database>) -> $crate::database::Result<Vec<$export>> {
                db.into().sql($query)
                    $(.bind((stringify!($binding), $binding)))*
                    .fetch()
                    .await
            }
        }
    };
}
