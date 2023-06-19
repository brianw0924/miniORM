use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};


/*
在這種情況下，它應該是安全的，因為我們在編譯時確定了表和欄位的名稱，
 */
fn rust_type_to_sql(ty: &Type) -> &'static str {
    match ty {
        Type::Path(p) => {
            let ident = &p.path.segments.last().unwrap().ident;
            match ident.to_string().as_str() {
                "i32" => "INTEGER",
                "i64" => "BIGINT",
                "f32" => "REAL",
                "f64" => "DOUBLE PRECISION",
                "String" => "TEXT",
                other_ty => panic!("Unsupported type {}", other_ty),
            }
        }
        _ => panic!("Unsupported type"),
    }
}

#[proc_macro_derive(CreateTable)]
pub fn create_table_macro(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let fields = match data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    let field_definitions: Vec<_> = fields
        .iter()
        .map(|field| {
            let ident = &field.ident.as_ref().unwrap();
            let ty = rust_type_to_sql(&field.ty);
            format!("{} {}", ident, ty)
        })
        .collect();

    let query = format!(
        "CREATE TABLE IF NOT EXISTS {} ( {} )",
        ident,
        field_definitions.join(", ")
    );

    let output = quote! {
        impl #ident {
            pub async fn create_table(pool: &sqlx::PgPool) -> sqlx::Result<()> {
                sqlx::query(#query)
                    .execute(pool)
                    .await?;
                Ok(())
            }
        }
    };

    TokenStream::from(output)
}

#[proc_macro_derive(Insert)]
pub fn insert_macro(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let fields = match data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    let field_names: Vec<_> = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect();

    let placeholders: Vec<_> = (1..=field_names.len())
        .map(|i| format!("${}", i))
        .collect();

    let query = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        ident,
        field_names
            .iter()
            .map(|ident| ident.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        placeholders.join(", ")
    );

    let output = quote! {
        impl #ident {
            pub async fn insert(pool: &sqlx::PgPool, item: &Self) -> sqlx::Result<()> {
                sqlx::query(&#query)
                    #( .bind(&item.#field_names) )*
                    .execute(pool)
                    .await?;
                Ok(())
            }
        }
    };

    TokenStream::from(output)
}


#[proc_macro_derive(Filter)]
pub fn filter_macro(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let query_builder_ident = Ident::new(&format!("{}QueryBuilder", ident), Span::call_site());

    let fields = match data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    let field_methods = fields.iter().map(|field| {
        let field_ident = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        let type_ident= if let Type::Path(p) = field_ty {
            &p.path.segments.last().unwrap().ident
        } else {
            panic!("Unsupported type");
        };
        quote! {
            pub fn #field_ident(mut self, value: #type_ident) -> Self {
                self.conditions.push(format!("{} = ${}", stringify!(#field_ident), self.param_count));
                self.bind_params.push(BindParamHolder::#type_ident(value));
                self.param_count += 1;
                self
            }
        }
    });

    let output = quote! {

        pub enum BindParamHolder {
            i32(i32),
            i64(i64),
            f32(f32),
            f64(f64),
            String(String)
        }

        pub struct #query_builder_ident {
            conditions: Vec<String>,
            bind_params: Vec<BindParamHolder>,
            param_count: usize,
        }

        impl #query_builder_ident {
            pub fn new() -> Self {
                Self {
                    conditions: vec![],
                    bind_params: vec![],
                    param_count: 1,
                }
            }

            #(#field_methods)*

            pub async fn select(&self, pool: &sqlx::PgPool) -> sqlx::Result<Vec<#ident>> {

                let conditions = self.conditions.join(" AND ");
                let query = format!("SELECT * FROM {} WHERE {}", stringify!(#ident), conditions);

                let mut sqlx_query = sqlx::query_as::<_, #ident>(&query);

                // Need better implementation to deal with different type binding paramters
                for param in self.bind_params.iter() {
                    match param {
                        BindParamHolder::i32(value) => {
                            sqlx_query = sqlx_query.bind(value);
                        },
                        BindParamHolder::String(value) => {
                            sqlx_query = sqlx_query.bind(value);
                        },
                        _ => {
                            println!("Unsupported type");
                        }
                    }
                }
                sqlx_query.fetch_all(pool).await
            }

            pub async fn delete(&self, pool: &sqlx::PgPool) -> sqlx::Result<Vec<#ident>> {

                let conditions = self.conditions.join(" AND ");
                let query = format!("DELETE FROM {} WHERE {}", stringify!(#ident), conditions);

                let mut sqlx_query = sqlx::query_as::<_, #ident>(&query);

                // Need better implementation to deal with different type binding paramters
                for param in self.bind_params.iter() {
                    match param {
                        BindParamHolder::i32(value) => {
                            sqlx_query = sqlx_query.bind(value);
                        },
                        BindParamHolder::String(value) => {
                            sqlx_query = sqlx_query.bind(value);
                        },
                        _ => {
                            println!("Unsupported type");
                        }
                    }
                }
                sqlx_query.fetch_all(pool).await
            }

        }


        impl #ident {
            pub fn filter() -> #query_builder_ident {
                #query_builder_ident::new()
            }
            
            pub async fn select(pool: &sqlx::PgPool) -> sqlx::Result<Vec<#ident>> {
                let query = format!("SELECT * FROM {}", stringify!(#ident));
                sqlx::query_as::<_, #ident>(&query)
                .fetch_all(pool)
                .await
            }

            pub async fn delete(pool: &sqlx::PgPool) -> sqlx::Result<Vec<#ident>> {
                let query = format!("DELETE FROM {}", stringify!(#ident));
                sqlx::query_as::<_, #ident>(&query)
                .fetch_all(pool)
                .await
            }

        }

    };

    TokenStream::from(output)
}








