#[macro_use] extern crate syn;
#[macro_use] extern crate quote;
extern crate proc_macro2;
use proc_macro::TokenStream;
use quote::{ToTokens};
use syn::{
	parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Data, DeriveInput, Error, ExprAssign, LitInt, LitStr, Meta, MetaNameValue, Path
};
mod symbol;
use symbol::Symbol;
use std::{iter::repeat, str::FromStr};
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
    let enum_val = contracts.iter().map(|v|
    {
        let mut token_string = String::new();
        if v.named_fields_types.len() > 0
        {
            token_string += "(v)";
            proc_macro2::TokenStream::from_str(&token_string).unwrap()
        }
        else 
        {
            token_string += "";
            proc_macro2::TokenStream::from_str(&token_string).unwrap()
        }
    });
    eprint!("{:?}", arr);
	return quote!(
        impl #name
        {
            /// sjhdfjsd sdjfksjdf sjdfks jskdjfk jsndfkj
            ///let arr = [#(#arr)*].to_vec();
            pub fn get_test(&self) -> String
            {
                let m = match *self
                {
                    #(#enum_names::#enum_vars #enum_val => "dummy",)*
                };
                m.to_owned()
            }
		}
	).into();
}



fn ts()
{

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
                            // Variant can have unnamed fields like `Variant(i32, i64)`
                            syn::Fields::Named(n) => 
                            {
                                for field in &n.named
                                {
                                    contract.named_fields_types.push(field.ty.clone());
                                }
                            },
                            // Variant can have named fields like `Variant {x: i32, y: i32}`
                            syn::Fields::Unnamed(un) => 
                            {
                                for field in &un.unnamed
                                {
                                    contract.unnamed_fields.push((field.ident.as_ref().unwrap().clone(), field.ty.clone()));
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
    ///типы именованых полей энума Test(String)
    named_fields_types: Vec<syn::Type>,
    ///имена:типы неименованных полей энума {a: i32}
    unnamed_fields: Vec<(syn::Ident, syn::Type)>
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
		parse_tokens(attributes)
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
                                        eprint!("{:?}", &contract.cmd);
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
    eprint!("{:?}", &contract);
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