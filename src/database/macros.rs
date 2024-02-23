#[macro_export]
macro_rules! table {
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
macro_rules! query {
    ($relation:ident ($($binding:ident : $binding_type:ty),*) -> $export:ty where $query:literal) => {
        #[tracing::instrument]
        pub async fn $relation($($binding : $binding_type ,)*) -> Result<$export, $crate::database::DatabaseError> {
            use $crate::database::Query;
            $crate::database::database()
                .query($query)
                $(.bind((stringify!($binding), $binding)))*
                .fetch()
                .await
        }
    };
}
