#[macro_export]
macro_rules! define_message {
    ($vis:vis msg $message:ident for $state:ty {
        $(
            $method:ident ($state_id:ident $(, $param:ident : $type:ty)* ) -> $ret:ty $body:block
        )*
    }) => {
        #[derive(Debug, Clone, PartialEq)]
        #[allow(non_camel_case_types)]
        $vis enum $message {
            $(
                $method { $($param : $type),* } ,
            )*
        }

        impl $message {
            #[allow(unused_variables)]
            pub async fn handle(self, state: &mut $state) -> $crate::service::watcher::Result<()> {
                match self {
                    $(
                        Self::$method { $($param),* } => Self::$method(state $(, $param)*).await
                    ),*
                }
            }

            $(
                pub async fn $method($state_id: &mut $state, $($param : $type),*) -> $ret {
                    $body
                }
            )*
        }
    };
}

pub use crate::define_message;
