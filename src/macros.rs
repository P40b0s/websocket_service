#[macro_export]
///Макрос для автоимлементации трейта PayloadType
macro_rules! impl_name {
        ($name: ident) => {
            
            impl  crate::server::PayloadType for $name
            {
                fn get_type() -> String
                {
                    stringify!($name).to_owned()
                }
            }
        }
    }