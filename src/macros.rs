#[macro_export]
macro_rules! impl_name {
        ($name: ident) => {
            //если делать декларации тут то они при кодогенерации будут дублироваться
            impl  PayloadType for $name
            {
                fn get_type() -> String
                {
                    stringify!($name).to_owned()
                }
            }
            // impl $name
            // {
            //     pub fn where_not_in(users: Vec<String>) -> String
            //     {
            //         let ids : String = users.into_iter().map(|m| ["\"".to_owned(), m, "\"".to_owned()].concat()).collect::<Vec<String>>().join(",");
            //         let id_in = [" WHERE id NOT IN ", "(", &ids, ")"].concat();
            //         id_in
            //     }
            // }
        }
    }