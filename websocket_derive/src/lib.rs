#[macro_use] extern crate syn;
#[macro_use] extern crate quote;
extern crate proc_macro2;
use proc_macro::TokenStream;
use quote::{ToTokens};
use syn::{
	parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Data, DeriveInput, Error, ExprAssign, Field, Ident, LitInt, LitStr, Meta, MetaNameValue, Path, Type, TypePath
};
mod symbol;
use symbol::Symbol;
use std::{any::Any, iter::repeat, str::FromStr};
/// The macro compile TokensDefinitions for Lexer.
///
/// An example of this:
/// ```rust
/// #[derive(Contract)]
/// pub enum TestTokens {
/// 	//pattern to recognize
///     #[contract(command="com")]
///     NewMessage,
/// ```
#[proc_macro_derive(Contract, attributes(contract, command, is_error))]
pub fn derive_contract(inp: TokenStream) -> TokenStream 
{
	let input = parse_macro_input!(inp as DeriveInput);
	let name = input.ident.clone();
	let mut arr: Vec<proc_macro2::TokenStream> = vec![];
    let parsed = parse(input);
    if let Err(e) = parsed
    {
        return e;
    }
    
    let contracts = parsed.unwrap();
    let enum_names = repeat(&name);
    let enum_vars = contracts.iter().map(|v| v.variant_name.as_ref().unwrap());
    eprintln!("{:?}", &contracts);
    let enum_val = contracts.iter().map(|v|
    {
        let mut token_string = String::new();
        eprint!("перебираем все типы...");
        if v.unnamed_fields.len() > 0
        {
            let ts = ts_with_data(&v.unnamed_fields);
            if let Err(e) = ts.as_ref()
            {
                eprintln!("{:?}", &e);
                syn::Error::to_compile_error(&e);
            }
            let ts = ts.unwrap();
            print!("преобразование объекта: {}", ts.to_string());
            ts
        }
        else 
        {
            token_string += " => println!(\"dummy\")";
            //proc_macro2::TokenStream::from_str(&token_string).unwrap()
            let ts = ts_empty();
            eprint!("преобразование объекта: {}", ts.to_string());
            ts
        }
    });
	return quote!(
        impl #name
        {
            /// sjhdfjsd sdjfksjdf sjdfks jskdjfk jsndfkj
            ///let arr = [#(#arr)*].to_vec();
            pub fn get_test(&self)
            {
                let m = match self
                {
                    #(#enum_names::#enum_vars #enum_val,)*
                };
                //m.to_owned();
            }
		}
	).into();
}



fn ts_with_data(fields: &Vec<Ident>) -> Result<proc_macro2::TokenStream, syn::Error>
{
    if fields.len() > 3
    {
        return Err(Error::new_spanned(fields.first().unwrap(), "В неимнованных типах поддерживается не более 3 типов"))
    }
    let lit =  r#"(v) => 
    {
        println!("{}", v);
    }"#;
   Ok(proc_macro2::TokenStream::from_str(lit).unwrap())
}
fn ts_empty() -> proc_macro2::TokenStream
{
    let lit =  r#" => 
    {
        println!("empty");
    }"#;
    proc_macro2::TokenStream::from_str(lit).unwrap()
}

fn parse(input: DeriveInput) -> Result<Vec<ContractAttributes>, TokenStream>
{
    let mut loc_arr: Vec<ContractAttributes> = vec![];
    match input.data
	{
		Data::Struct(_) =>unimplemented!("Struct"),
		Data::Union(_) => unimplemented!("Union"),
		Data::Enum(e) => 
		{
			for var in  &e.variants
			{
				if var.attrs.len() > 0
				{
					let mut contract = ContractAttributes::new(&var.attrs);
                    if let Ok(contract) = contract.as_mut()
                    {   
                        contract.variant_name = Some(var.ident.clone());
                        match &var.fields
                        {
                            // Variant can have unnamed fields like `Variant(i32, i64)`
                            // Variant can have named fields like `Variant {x: i32, y: i32}`
                            // Variant can be named Unit like `Variant`
                            // syn::Fields::Named(n) => quote_spanned! {var.span() => (..)},
                            // syn::Fields::Unnamed(un) => quote_spanned! {var.span() => (..)},
                            // syn::Fields::Unit =>  quote_spanned! {var.span() => (..)},
                            // Variant can have named fields like `Variant {x: i32, y: i32}`
                            syn::Fields::Named(n) => 
                            {
                                for field in &n.named
                                {
                                    eprintln!("парсинг полей именованного энум: {:?}", &field);
                                    //TODO не сдалано!
                                    //contract.named_fields_types.push(field.ty.clone());
                                }
                            },
                            
                            // Variant can have unnamed fields like `Variant(i32, i64)`
                            syn::Fields::Unnamed(un) => 
                            {
                                for field in &un.unnamed
                                {
                                    eprintln!("парсинг полей неименованного энум: {:?}", &field);
                                    if let Type::Path(p) = &field.ty
                                    {
                                       for s in &p.path.segments
                                       {
                                            contract.unnamed_fields.push(s.ident.clone())
                                       }
                                    }
                                }
                            },
                            // Variant can be named Unit like `Variant`
                            //нет не типа ни имени поэтому никак не обрабатываем
                            syn::Fields::Unit =>  (),
                        };
                        loc_arr.push(contract.clone());
                    }
                    else 
                    {
                        return Err(contract.err().unwrap().to_compile_error().into());
                    }
                   
                    //arr.push(quote!(contract));
					//let variant_ident = var.ident.clone();
					//println!("{:?}", &defs);
				// 	for def in defs
				// 	{
						
				// 		let mut pattern: &String = &String::new();
				// 		let mut pr = 0u8;
				// 		if let Some(p) = def.pattern.as_ref()
				// 		{
				// 			pattern = p;
				// 		}
				// 		else
				// 		{
				// 			eprintln!("Вы не указали pattern для {}", &var.ident.to_string());
				// 			return var.ident.to_token_stream().into();
				// 		}
				// 		if def.precedence.is_some()
				// 		{
				// 			pr = def.precedence.unwrap();
				// 		}
				// 		// if let Some(conv) = def.split_conv
				// 		// {
				// 		// 	let c1 = conv.0;
				// 		// 	let c2 = conv.1;
				// 		// 	let rr = quote!(::tokenizer::TokenDefinition::<#name>::new(#name::#enu, #pattern, #pr, Some([#c1, #c2])),);
				// 		// 	arr.push(rr);
				// 		// }
				// 		if let Some(conv) = def.converter
				// 		{
				// 			let rr = quote!(::tokenizer::TokenDefinition::<#name>::new(#name::#contract.variant_name, #pattern, #pr, Some(#conv.to_string())),);
				// 			arr.push(rr);
				// 		}
				// 		else 
				// 		{
				// 			let rr = quote!(::tokenizer::TokenDefinition::<#name>::new(#name::#enu, #pattern, #pr, None),);
				// 			arr.push(rr);
				// 		}
				// 	}
				}
			};
		}
	};
    Ok(loc_arr)
}

#[derive(Debug, Clone)]
struct ContractAttributes
{
	cmd: Option<String>,
    is_error: bool,
    variant_name: Option<syn::Ident>,
    
    ///имена:типы именованных полей энума {a: i32}
    named_fields_types: Vec<(syn::Ident, syn::Type)>,
    ///типы неименованных полей энума Test(String)
    unnamed_fields: Vec<syn::Ident>,
}
impl Default for ContractAttributes
{
	fn default() -> Self 
	{
		ContractAttributes 
        {
            cmd: None,
            is_error: false,
            variant_name: None,
            named_fields_types: vec![],
            unnamed_fields: vec![]
        }
	}
}

impl ContractAttributes
{
	pub fn new(attributes: &[Attribute]) -> Result<ContractAttributes, syn::Error>
    {
        let parsed = parse_tokens(attributes);
		parsed
	}
}

fn parse_tokens(attributes: &[Attribute]) -> Result<ContractAttributes, syn::Error> 
{
    let mut contract = ContractAttributes::default();
	for attr in attributes
	{
		if attr.path().is_ident(&symbol::BASE)
		{
			let nested = attr.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)?;
			{
				for meta in nested 
				{
					match &meta 
					{
						// #[contract(cmd = "test")]
						MetaNameValue { path, eq_token, value } if meta.path.is_ident(&symbol::COMMAND) => 
						{
                            match value
                            {
                                syn::Expr::Lit(expr) => 
                                {
                                    if let syn::Lit::Str(lit_str) = &expr.lit
                                    {
                                        contract.cmd = Some(lit_str.value());
                                    }
                                }
                                _=> return Err(Error::new_spanned(meta, "Поддерживаются только литеральные значения cmd = \"task\\update\""))
                            }
						},
                        MetaNameValue { path, eq_token, value } if meta.path.is_ident(&symbol::IS_ERROR) => 
						{
                            match value
                            {
                                syn::Expr::Lit(expr) => 
                                {
                                    if let syn::Lit::Bool(lit_bool) = &expr.lit
                                    {
                                        contract.is_error = lit_bool.value();
                                    }
                                }
                                _=> return Err(Error::new_spanned(meta, "Поддерживаются только литеральные значения cmd = \"task\\update\""))
                            }
						},
						_ => 
						{
							return Err(Error::new_spanned(meta, "Не найден атрибут"));
						}
					}
				}
			}
		}
	}
    if contract.cmd.is_none()
    {
        return Err(Error::new_spanned(attributes.get(1), "Атрибуты для правильного формирования контракта не найдены"));
    }
	Ok(contract)
}


// #[proc_macro_attribute]
// pub fn contract(args: TokenStream, input: TokenStream) -> proc_macro::TokenStream 
// {
//     let mut kind: Option<LitStr> = None;
//     let mut hot: bool = false;
//     let mut with: Vec<Path> = Vec::new();
//     let contract_parser = syn::meta::parser(|meta| 
//     {
//         if meta.path.is_ident("command") 
//         {
//             kind = Some(meta.value()?.parse()?);
//             Ok(())
//         } 
//         else if meta.path.is_ident("hot") 
//         {
//             hot = true;
//             Ok(())
//         } 
//         else if meta.path.is_ident("with") 
//         {
//             meta.parse_nested_meta(|meta| 
//             {
//                 with.push(meta.path);
//                 Ok(())
//             })
//         } 
//         else 
//         {
//             Err(meta.error("unsupported tea property"))
//         }
//     });

//     parse_macro_input!(args with contract_parser);
//     eprintln!("kind={kind:?} hot={hot} with={with:?}");

//     /* ... */
// }