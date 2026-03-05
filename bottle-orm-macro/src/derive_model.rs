//! # Model Derive Macro Implementation
//!
//! This module implements the procedural macro expansion for `#[derive(Model)]`.
//! It analyzes struct fields and their `#[orm(...)]` attributes to automatically
//! generate the `Model` trait implementation.

// ============================================================================
// External Crate Imports
// ============================================================================

use proc_macro2::TokenStream;
use quote::{quote, format_ident};
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};
use heck::ToSnakeCase;

// ============================================================================
// Internal Crate Imports
// ============================================================================

use crate::types::rust_type_to_sql;

// ============================================================================
// Helper Functions
// ============================================================================

/// Extracts the inner type `T` from `Option<T>`.
fn get_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

// ============================================================================
// Macro Expansion Function
// ============================================================================

/// Expands the `#[derive(Model)]` macro.
pub fn expand(ast: DeriveInput) -> TokenStream {
    let struct_name = &ast.ident;

    let fields = if let Data::Struct(data) = &ast.data {
        if let Fields::Named(fields) = &data.fields {
            fields
        } else {
            panic!("Model must have named fields");
        }
    } else {
        panic!("Model must be a struct")
    };

    let mut table_name_str = struct_name.to_string().to_snake_case();
    for attr in &ast.attrs {
        if attr.path().is_ident("orm") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("table") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    table_name_str = value.value();
                }
                Ok(())
            });
        }
    }

    let mut relations = Vec::new();

    let column_defs_iter = fields.named.iter().filter_map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        let (mut sql_type, is_nullable) = rust_type_to_sql(field_type);

        let mut is_primary_key = false;
        let mut size = None;
        let mut create_time = false;
        let mut update_time = false;
        let mut unique = false;
        let mut index = false;
        let mut omit = false;
        let mut soft_delete = false;
        let mut is_enum = false;
        let mut foreign_table_tokens = quote! { None };
        let mut foreign_key_tokens = quote! { None };

        let mut rel_type = None;
        let mut rel_target = None;
        let mut rel_fk = None;
        let mut rel_lk = None;

        for attr in &f.attrs {
            if attr.path().is_ident("orm") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("primary_key") { is_primary_key = true; }
                    if meta.path.is_ident("size") {
                        let value: syn::LitInt = meta.value()?.parse()?;
                        size = Some(value.base10_parse::<usize>()?);
                    }
                    if meta.path.is_ident("create_time") { create_time = true; }
                    if meta.path.is_ident("update_time") { update_time = true; }
                    if meta.path.is_ident("unique") { unique = true; }
                    if meta.path.is_ident("index") { index = true; }
                    if meta.path.is_ident("foreign_key") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        let fk_string = value.value();
                        if fk_string.contains("::") {
                            let parts: Vec<&str> = fk_string.split("::").collect();
                            if parts.len() == 2 {
                                let table = parts[0];
                                let col = parts[1];
                                foreign_table_tokens = quote! { Some(#table) };
                                foreign_key_tokens = quote! { Some(#col) };
                            }
                        } else { rel_fk = Some(fk_string); }
                    }
                    if meta.path.is_ident("local_key") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        rel_lk = Some(value.value());
                    }
                    if meta.path.is_ident("has_many") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        rel_type = Some(quote! { bottle_orm::RelationType::HasMany });
                        rel_target = Some(value.value());
                    }
                    if meta.path.is_ident("has_one") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        rel_type = Some(quote! { bottle_orm::RelationType::HasOne });
                        rel_target = Some(value.value());
                    }
                    if meta.path.is_ident("belongs_to") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        rel_type = Some(quote! { bottle_orm::RelationType::BelongsTo });
                        rel_target = Some(value.value());
                    }
                    if meta.path.is_ident("omit") { omit = true; }
                    if meta.path.is_ident("soft_delete") { soft_delete = true; }
                    if meta.path.is_ident("enum") { is_enum = true; }
                    Ok(())
                });
            }
        }

        if let Some(rtype) = rel_type {
            let target = rel_target.unwrap();
            let fk = rel_fk.unwrap_or_else(|| "id".to_string());
            let lk = rel_lk.unwrap_or_else(|| "id".to_string());
            let field_name_str = field_name.as_ref().unwrap().to_string();
            relations.push(quote! {
                bottle_orm::RelationInfo {
                    name: #field_name_str,
                    rel_type: #rtype,
                    target_table: #target,
                    foreign_key: #fk,
                    local_key: #lk,
                }
            });
            return None;
        }

        if let Some(s) = size { if sql_type == "TEXT" { sql_type = format!("VARCHAR({})", s); } }
        if is_enum && (sql_type == "TEXT" || sql_type == "VARCHAR(255)") { sql_type = "TEXT".to_string(); }

        Some(quote! {
            bottle_orm::ColumnInfo {
                 name: stringify!(#field_name),
                 sql_type: #sql_type,
                 is_primary_key: #is_primary_key,
                 is_nullable: #is_nullable,
                 create_time: #create_time,
                 update_time: #update_time,
                 unique: #unique,
                 index: #index,
                 foreign_table: #foreign_table_tokens,
                 foreign_key: #foreign_key_tokens,
                 omit: #omit,
                 soft_delete: #soft_delete,
            }
        })
    });

    let column_defs: Vec<_> = column_defs_iter.collect();

    let load_relations_arms = fields.named.iter().filter_map(|f| {
        let field_name = &f.ident;
        let mut rel_type = None;
        let mut rel_target = None;
        let mut rel_fk = None;
        let mut rel_lk = None;

        for attr in &f.attrs {
            if attr.path().is_ident("orm") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("has_many") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        rel_type = Some("HasMany"); rel_target = Some(value.value());
                    } else if meta.path.is_ident("has_one") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        rel_type = Some("HasOne"); rel_target = Some(value.value());
                    } else if meta.path.is_ident("belongs_to") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        rel_type = Some("BelongsTo"); rel_target = Some(value.value());
                    }
                    if meta.path.is_ident("foreign_key") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        let s = value.value(); if !s.contains("::") { rel_fk = Some(s); }
                    }
                    if meta.path.is_ident("local_key") {
                        let value: syn::LitStr = meta.value()?.parse()?; rel_lk = Some(value.value());
                    }
                    Ok(())
                });
            }
        }

        if let Some(rtype) = rel_type {
            let target_ident = format_ident!("{}", rel_target.unwrap());
            let target_table_name = target_ident.to_string().to_snake_case();
            let fk = rel_fk.unwrap_or_else(|| "id".to_string());
            let lk = rel_lk.unwrap_or_else(|| "id".to_string());
            let lk_ident = format_ident!("{}", lk);
            let fk_ident = format_ident!("{}", fk);
            let fk_str = fk.to_snake_case();
            let field_name_str = field_name.as_ref().unwrap().to_string();

            if rtype == "HasMany" {
                return Some(quote! {
                    #field_name_str => {
                        use sqlx::Arguments;
                        let ids: Vec<_> = models.iter().map(|m| m.#lk_ident.clone()).collect();
                        if ids.is_empty() { return Ok(()); }
                        let target_table_real = <#target_ident as bottle_orm::Model>::table_name();
                        let mut query = format!("SELECT * FROM \"{}\" WHERE \"{}\" IN (", target_table_real, #fk_str);
                        let mut args = sqlx::any::AnyArguments::default();
                        let mut placeholders = Vec::new();
                        for (i, id) in ids.iter().enumerate() {
                            match tx.driver() {
                                bottle_orm::database::Drivers::Postgres => placeholders.push(format!("${}", i + 1)),
                                _ => placeholders.push("?".to_string()),
                            }
                            let _ = args.add(id.clone());
                        }
                        query.push_str(&placeholders.join(", "));
                        query.push_str(")");
                        let mut arg_counter = ids.len() + 1;
                        let mut sub_modifier = None;
                        if let Some(modifier) = query_modifier.clone() {
                             let temp_builder = bottle_orm::query_builder::QueryBuilder::<#target_ident, bottle_orm::Database>::new(
                                 tx.clone_db(), tx.driver(), <#target_ident as bottle_orm::Model>::table_name(),
                                 <#target_ident as bottle_orm::Model>::columns(), <#target_ident as bottle_orm::Model>::active_columns().iter().map(|s| s.to_string()).collect(),
                             );
                             let any_builder = unsafe { std::mem::transmute::<bottle_orm::query_builder::QueryBuilder<#target_ident, bottle_orm::Database>, bottle_orm::query_builder::QueryBuilder<bottle_orm::any_struct::AnyImplStruct, bottle_orm::Database>>(temp_builder) };
                             let mut modified_builder = if let Some(wrapper) = modifier.downcast_ref::<bottle_orm::query_builder::QueryModifier>() {
                                 (wrapper.modifier)(any_builder)
                             } else if let Some(wrapper) = modifier.downcast_ref::<std::sync::Arc<bottle_orm::query_builder::QueryModifier>>() {
                                 (wrapper.modifier)(any_builder)
                             } else {
                                 any_builder
                             };
                             for clause in modified_builder.where_clauses { clause(&mut query, &mut args, &tx.driver(), &mut arg_counter); }
                             if !modified_builder.order_clauses.is_empty() { query.push_str(&format!(" ORDER BY {}", modified_builder.order_clauses.join(", "))); }
                             if let Some(limit) = modified_builder.limit {
                                 query.push_str(" LIMIT ");
                                 match tx.driver() { bottle_orm::database::Drivers::Postgres => { query.push_str(&format!("${}", arg_counter)); arg_counter += 1; } _ => query.push('?'), }
                                 let _ = args.add(limit as i64);
                             }
                             if let Some(offset) = modified_builder.offset {
                                 query.push_str(" OFFSET ");
                                 match tx.driver() { bottle_orm::database::Drivers::Postgres => { query.push_str(&format!("${}", arg_counter)); arg_counter += 1; } _ => query.push('?'), }
                                 let _ = args.add(offset as i64);
                             }
                             if !nested_rel.is_empty() { sub_modifier = modified_builder.with_modifiers.remove(nested_rel); }
                        } else if !options.is_empty() {
                            let (mut order_clause, mut limit_clause, mut offset_clause) = (String::new(), String::new(), String::new());
                            for opt in options.split(',') {
                                let opt = opt.trim();
                                if let Some(val) = opt.strip_prefix("limit=") { limit_clause = format!(" LIMIT {}", val); }
                                else if let Some(val) = opt.strip_prefix("offset=") { offset_clause = format!(" OFFSET {}", val); }
                                else if let Some(val) = opt.strip_prefix("order=") { order_clause = format!(" ORDER BY {}", val); }
                            }
                            query.push_str(&order_clause); query.push_str(&limit_clause); query.push_str(&offset_clause);
                        }
                        let rows = tx.fetch_all(&query, args).await?;
                        let mut related: Vec<#target_ident> = rows.iter().map(|r| <#target_ident as sqlx::FromRow<sqlx::any::AnyRow>>::from_row(r)).collect::<Result<Vec<_>, _>>()?;
                        if !nested_rel.is_empty() { <#target_ident as bottle_orm::Model>::load_relations(nested_rel, &mut related, tx, sub_modifier).await?; }
                        for model in models.iter_mut() {
                            model.#field_name = related.iter().filter(|r| r.#fk_ident.to_string() == model.#lk_ident.to_string()).cloned().collect();
                        }
                    }
                });
            } else if rtype == "HasOne" {
                return Some(quote! {
                    #field_name_str => {
                        use sqlx::Arguments;
                        let ids: Vec<_> = models.iter().map(|m| m.#lk_ident.clone()).collect();
                        if ids.is_empty() { return Ok(()); }
                        let target_table_real = <#target_ident as bottle_orm::Model>::table_name();
                        let mut query = format!("SELECT * FROM \"{}\" WHERE \"{}\" IN (", target_table_real, #fk_str);
                        let mut args = sqlx::any::AnyArguments::default();
                        let mut placeholders = Vec::new();
                        for (i, id) in ids.iter().enumerate() {
                            match tx.driver() { bottle_orm::database::Drivers::Postgres => placeholders.push(format!("${}", i + 1)), _ => placeholders.push("?".to_string()), }
                            let _ = args.add(id.clone());
                        }
                        query.push_str(&placeholders.join(", ")); query.push_str(")");
                        let mut arg_counter = ids.len() + 1;
                        let mut sub_modifier = None;
                        if let Some(modifier) = query_modifier.clone() {
                             let temp_builder = bottle_orm::query_builder::QueryBuilder::<#target_ident, bottle_orm::Database>::new(
                                 tx.clone_db(), tx.driver(), <#target_ident as bottle_orm::Model>::table_name(),
                                 <#target_ident as bottle_orm::Model>::columns(), <#target_ident as bottle_orm::Model>::active_columns().iter().map(|s| s.to_string()).collect(),
                             );
                             let any_builder = unsafe { std::mem::transmute::<bottle_orm::query_builder::QueryBuilder<#target_ident, bottle_orm::Database>, bottle_orm::query_builder::QueryBuilder<bottle_orm::any_struct::AnyImplStruct, bottle_orm::Database>>(temp_builder) };
                             let mut modified_builder = if let Some(wrapper) = modifier.downcast_ref::<bottle_orm::query_builder::QueryModifier>() {
                                 (wrapper.modifier)(any_builder)
                             } else if let Some(wrapper) = modifier.downcast_ref::<std::sync::Arc<bottle_orm::query_builder::QueryModifier>>() {
                                 (wrapper.modifier)(any_builder)
                             } else {
                                 any_builder
                             };
                             for clause in modified_builder.where_clauses { clause(&mut query, &mut args, &tx.driver(), &mut arg_counter); }
                             if !modified_builder.order_clauses.is_empty() { query.push_str(&format!(" ORDER BY {}", modified_builder.order_clauses.join(", "))); }
                             if !nested_rel.is_empty() { sub_modifier = modified_builder.with_modifiers.remove(nested_rel); }
                        } else if !options.is_empty() {
                            for opt in options.split(',') {
                                let opt = opt.trim();
                                if let Some(val) = opt.strip_prefix("order=") { query.push_str(&format!(" ORDER BY {}", val)); }
                            }
                        }
                        let rows = tx.fetch_all(&query, args).await?;
                        let mut related: Vec<#target_ident> = rows.iter().map(|r| <#target_ident as sqlx::FromRow<sqlx::any::AnyRow>>::from_row(r)).collect::<Result<Vec<_>, _>>()?;
                        if !nested_rel.is_empty() { <#target_ident as bottle_orm::Model>::load_relations(nested_rel, &mut related, tx, sub_modifier).await?; }
                        for model in models.iter_mut() {
                            model.#field_name = related.iter().find(|r| r.#fk_ident.to_string() == model.#lk_ident.to_string()).cloned();
                        }
                    }
                });
            } else if rtype == "BelongsTo" {
                 let local_fk_ident = format_ident!("{}", fk);
                 let target_lk_str = lk.to_snake_case();
                 let target_lk_ident = format_ident!("{}", lk);
                 return Some(quote! {
                    #field_name_str => {
                        use sqlx::Arguments;
                        let ids: Vec<_> = models.iter().filter_map(|m| { let val = m.#local_fk_ident.to_string(); if val == "None" || val.is_empty() { None } else { Some(m.#local_fk_ident.clone()) } }).collect();
                        if ids.is_empty() { return Ok(()); }
                        let target_table_real = <#target_ident as bottle_orm::Model>::table_name();
                        let mut query = format!("SELECT * FROM \"{}\" WHERE \"{}\" IN (", target_table_real, #target_lk_str);
                        let mut args = sqlx::any::AnyArguments::default();
                        let mut placeholders = Vec::new();
                        for (i, id) in ids.iter().enumerate() {
                            match tx.driver() { bottle_orm::database::Drivers::Postgres => placeholders.push(format!("${}", i + 1)), _ => placeholders.push("?".to_string()), }
                            let _ = args.add(id.clone());
                        }
                        query.push_str(&placeholders.join(", ")); query.push_str(")");
                        let mut arg_counter = ids.len() + 1;
                        let mut sub_modifier = None;
                        if let Some(modifier) = query_modifier.clone() {
                             let temp_builder = bottle_orm::query_builder::QueryBuilder::<#target_ident, bottle_orm::Database>::new(
                                 tx.clone_db(), tx.driver(), <#target_ident as bottle_orm::Model>::table_name(),
                                 <#target_ident as bottle_orm::Model>::columns(), <#target_ident as bottle_orm::Model>::active_columns().iter().map(|s| s.to_string()).collect(),
                             );
                             let any_builder = unsafe { std::mem::transmute::<bottle_orm::query_builder::QueryBuilder<#target_ident, bottle_orm::Database>, bottle_orm::query_builder::QueryBuilder<bottle_orm::any_struct::AnyImplStruct, bottle_orm::Database>>(temp_builder) };
                             let mut modified_builder = if let Some(wrapper) = modifier.downcast_ref::<bottle_orm::query_builder::QueryModifier>() {
                                 (wrapper.modifier)(any_builder)
                             } else if let Some(wrapper) = modifier.downcast_ref::<std::sync::Arc<bottle_orm::query_builder::QueryModifier>>() {
                                 (wrapper.modifier)(any_builder)
                             } else {
                                 any_builder
                             };
                             for clause in modified_builder.where_clauses { clause(&mut query, &mut args, &tx.driver(), &mut arg_counter); }
                             if !modified_builder.order_clauses.is_empty() { query.push_str(&format!(" ORDER BY {}", modified_builder.order_clauses.join(", "))); }
                             if !nested_rel.is_empty() { sub_modifier = modified_builder.with_modifiers.remove(nested_rel); }
                        } else if !options.is_empty() {
                            for opt in options.split(',') {
                                let opt = opt.trim();
                                if let Some(val) = opt.strip_prefix("order=") { query.push_str(&format!(" ORDER BY {}", val)); }
                            }
                        }
                        let rows = tx.fetch_all(&query, args).await?;
                        let mut related: Vec<#target_ident> = rows.iter().map(|r| <#target_ident as sqlx::FromRow<sqlx::any::AnyRow>>::from_row(r)).collect::<Result<Vec<_>, _>>()?;
                        if !nested_rel.is_empty() { <#target_ident as bottle_orm::Model>::load_relations(nested_rel, &mut related, tx, sub_modifier).await?; }
                        for model in models.iter_mut() {
                            model.#field_name = related.iter().find(|r| r.#target_lk_ident.to_string() == model.#local_fk_ident.to_string()).cloned();
                        }
                    }
                });
            }
        }
        None
    }).collect::<Vec<_>>();

    let field_names_iter: Vec<_> = fields.named.iter().filter(|f| {
        !f.attrs.iter().any(|attr| {
            if attr.path().is_ident("orm") {
                let mut is_rel = false;
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("has_many") || meta.path.is_ident("has_one") || meta.path.is_ident("belongs_to") { is_rel = true; }
                    Ok(())
                });
                is_rel
            } else { false }
        })
    }).map(|f| &f.ident).collect();

    let map_inserts = fields.named.iter().filter_map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        if f.attrs.iter().any(|attr| {
            if attr.path().is_ident("orm") {
                let mut is_rel = false;
                let _ = attr.parse_nested_meta(|meta| { if meta.path.is_ident("has_many") || meta.path.is_ident("has_one") || meta.path.is_ident("belongs_to") { is_rel = true; } Ok(()) });
                is_rel
            } else { false }
        }) { return None; }
        let (sql_type, is_nullable) = rust_type_to_sql(field_type);
        let is_complex = sql_type.ends_with("[]") || sql_type == "JSONB" || sql_type == "JSON";
        if is_nullable {
            return Some(quote! {
                map.insert(stringify!(#field_name).to_string(), self.#field_name.as_ref().map(|v| { if #is_complex { serde_json::to_string(v).unwrap_or_else(|_| "".to_string()) } else { v.to_string() } }));
            });
        }
        if is_complex {
            Some(quote! { map.insert(stringify!(#field_name).to_string(), Some(serde_json::to_string(&self.#field_name).unwrap_or_else(|_| "".to_string()))); })
        } else {
            Some(quote! { map.insert(stringify!(#field_name).to_string(), Some(self.#field_name.to_string())); })
        }
    });

    let any_column_defs = fields.named.iter().filter_map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        if f.attrs.iter().any(|attr| {
            if attr.path().is_ident("orm") {
                let mut is_rel = false;
                let _ = attr.parse_nested_meta(|meta| { if meta.path.is_ident("has_many") || meta.path.is_ident("has_one") || meta.path.is_ident("belongs_to") { is_rel = true; } Ok(()) });
                is_rel
            } else { false }
        }) { return None; }
        let (sql_type, _) = rust_type_to_sql(field_type);
        let table_name_const = table_name_str.clone();
        Some(quote! {
            bottle_orm::AnyInfo { column: stringify!(#field_name), sql_type: #sql_type, table: #table_name_const, }
        })
    });

    let from_row_logic = fields.named.iter().map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        let mut is_rel = false; let mut rel_type = None;
        for attr in &f.attrs {
            if attr.path().is_ident("orm") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("has_many") { is_rel = true; rel_type = Some("HasMany"); }
                    else if meta.path.is_ident("has_one") { is_rel = true; rel_type = Some("HasOne"); }
                    else if meta.path.is_ident("belongs_to") { is_rel = true; rel_type = Some("BelongsTo"); }
                    Ok(())
                });
            }
        }
        if is_rel {
            if rel_type == Some("HasMany") { return quote! { let #field_name: #field_type = Vec::new(); }; }
            else { return quote! { let #field_name: #field_type = None; }; }
        }
        let column_name = field_name.as_ref().unwrap().to_string();
        let alias_name = format!("{}__{}", table_name_str, column_name);
        let (sql_type, is_nullable) = rust_type_to_sql(field_type);
        let mut is_enum = false;
        for attr in &f.attrs { if attr.path().is_ident("orm") { let _ = attr.parse_nested_meta(|meta| { if meta.path.is_ident("enum") { is_enum = true; } Ok(()) }); } }
        if is_enum {
            if is_nullable {
                if let Some(inner_type) = get_inner_type(field_type) {
                    quote! {
                        let #field_name: #field_type = match row.try_get::<Option<String>, _>(#alias_name).or_else(|_| row.try_get::<Option<String>, _>(#column_name))? {
                            Some(s) => Some(s.parse::<#inner_type>().map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to parse enum: {}", e)))))?),
                            None => None,
                        };
                    }
                } else { quote! { let #field_name: #field_type = row.try_get(#alias_name).or_else(|_| row.try_get(#column_name))?; } }
            } else {
                quote! {
                    let #field_name: #field_type = {
                        let s: String = row.try_get(#alias_name).or_else(|_| row.try_get(#column_name))?;
                        s.parse().map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to parse enum: {}", e)))))?
                    };
                }
            }
        } else if sql_type == "TIMESTAMPTZ" || sql_type == "TIMESTAMP" || sql_type == "DATE" || sql_type == "TIME" {
             if is_nullable {
                 if let Some(inner_type) = get_inner_type(field_type) {
                     quote! {
                        let #field_name: #field_type = match row.try_get::<Option<String>, _>(#alias_name).or_else(|_| row.try_get::<Option<String>, _>(#column_name))? {
                            Some(s) => Some(s.parse::<#inner_type>().map_err(|e| sqlx::Error::Decode(Box::new(e)))?),
                            None => None,
                        };
                     }
                 } else { quote! { let #field_name: #field_type = row.try_get(#alias_name).or_else(|_| row.try_get(#column_name))?; } }
             } else {
                 quote! {
                    let #field_name: #field_type = {
                        let s: String = row.try_get(#alias_name).or_else(|_| row.try_get(#column_name))?;
                        s.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                    };
                 }
             }
        } else if sql_type == "UUID" {
             if is_nullable {
                 if let Some(inner_type) = get_inner_type(field_type) {
                     quote! {
                        let #field_name: #field_type = match row.try_get::<Option<String>, _>(#alias_name).or_else(|_| row.try_get::<Option<String>, _>(#column_name))? {
                            Some(s) => Some(s.parse::<#inner_type>().map_err(|e| sqlx::Error::Decode(Box::new(e)))?),
                            None => None,
                        };
                     }
                 } else { quote! { let #field_name: #field_type = row.try_get(#alias_name).or_else(|_| row.try_get(#column_name))?; } }
             } else {
                 quote! {
                    let #field_name: #field_type = {
                        let s: String = row.try_get(#alias_name).or_else(|_| row.try_get(#column_name))?;
                        s.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                    };
                 }
             }
        } else if sql_type.ends_with("[]") || sql_type == "JSONB" || sql_type == "JSON" {
            quote! {
                let #field_name: #field_type = {
                    let mut index = 0;
                    match row.try_column(#alias_name) {
                        Ok(col) => { index = sqlx::Column::ordinal(col); bottle_orm::any_struct::FromAnyRow::from_any_row_at(row, &mut index)? }
                        Err(_) => { let col = row.try_column(#column_name)?; index = sqlx::Column::ordinal(col); bottle_orm::any_struct::FromAnyRow::from_any_row_at(row, &mut index)? }
                    }
                };
            }
        } else { quote! { let #field_name: #field_type = row.try_get(#alias_name).or_else(|_| row.try_get(#column_name))?; } }
    });

    let from_row_logic_clone = from_row_logic.clone();
    let from_row_logic_positional = fields.named.iter().map(|f| {
        let field_name = &f.ident; let field_type = &f.ty;
        let mut is_rel = false; let mut rel_type = None;
        for attr in &f.attrs {
            if attr.path().is_ident("orm") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("has_many") { is_rel = true; rel_type = Some("HasMany"); }
                    else if meta.path.is_ident("has_one") { is_rel = true; rel_type = Some("HasOne"); }
                    else if meta.path.is_ident("belongs_to") { is_rel = true; rel_type = Some("BelongsTo"); }
                    Ok(())
                });
            }
        }
        if is_rel {
            if rel_type == Some("HasMany") { return quote! { let #field_name: #field_type = Vec::new(); }; }
            else { return quote! { let #field_name: #field_type = None; }; }
        }
        let (sql_type, is_nullable) = rust_type_to_sql(field_type);
        let mut is_enum = false;
        for attr in &f.attrs { if attr.path().is_ident("orm") { let _ = attr.parse_nested_meta(|meta| { if meta.path.is_ident("enum") { is_enum = true; } Ok(()) }); } }
        if is_enum {
            if is_nullable {
                if let Some(inner_type) = get_inner_type(field_type) {
                    quote! {
                        let #field_name: #field_type = {
                            let s: Option<String> = row.try_get(*index).map_err(|e| sqlx::Error::ColumnDecode { index: index.to_string(), source: Box::new(e) })?;
                            *index += 1;
                            match s { Some(s_val) => Some(s_val.parse::<#inner_type>().map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to parse enum: {}", e)))))?), None => None, }
                        };
                    }
                } else { quote! { let #field_name: #field_type = row.try_get(*index)?; *index += 1; } }
            } else {
                quote! {
                    let #field_name: #field_type = {
                        let s: String = row.try_get(*index).map_err(|e| sqlx::Error::ColumnDecode { index: index.to_string(), source: Box::new(e) })?;
                        *index += 1; s.parse().map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to parse enum: {}", e)))))?
                    };
                }
            }
        } else if sql_type == "TIMESTAMPTZ" || sql_type == "TIMESTAMP" || sql_type == "DATE" || sql_type == "TIME" || sql_type == "UUID" {
            if is_nullable {
                if let Some(inner_type) = get_inner_type(field_type) {
                    quote! {
                        let #field_name: #field_type = {
                            let s: Option<String> = row.try_get(*index).map_err(|e| sqlx::Error::ColumnDecode { index: index.to_string(), source: Box::new(e) })?;
                            *index += 1;
                            match s { Some(s_val) => Some(s_val.parse::<#inner_type>().map_err(|e| sqlx::Error::Decode(Box::new(e)))?), None => None, }
                        };
                    }
                } else { quote! { let #field_name: #field_type = row.try_get(*index)?; *index += 1; } }
            } else {
                quote! {
                    let #field_name: #field_type = {
                        let s: String = row.try_get(*index).map_err(|e| sqlx::Error::ColumnDecode { index: index.to_string(), source: Box::new(e) })?;
                        *index += 1; s.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                    };
                }
            }
        } else { quote! { let #field_name: #field_type = bottle_orm::any_struct::FromAnyRow::from_any_row_at(row, index)?; } }
    });

    let field_names_construct = fields.named.iter().map(|f| &f.ident);
    let field_names_construct_clone = field_names_construct.clone();
    let field_names_construct_positional = field_names_construct.clone();
    let module_name = format_ident!("{}_fields", struct_name.to_string().to_snake_case());
    let field_constants = fields.named.iter().filter_map(|f| {
        let field_name = &f.ident;
        let const_name = format_ident!("{}", field_name.as_ref().unwrap().to_string().to_uppercase());
        let name_str = field_name.as_ref().unwrap().to_string();
        Some(quote! { pub const #const_name: &'static str = #name_str; })
    });

    quote! {
        pub mod #module_name { #(#field_constants)* }
        impl bottle_orm::Model for #struct_name {
            fn table_name() -> &'static str { #table_name_str }
            fn columns() -> Vec<bottle_orm::ColumnInfo> { vec![#(#column_defs),*] }
            fn column_names() -> Vec<String> { vec![#(stringify!(#field_names_iter).to_string() ),*] }
            fn active_columns() -> Vec<&'static str> { vec![#(stringify!(#field_names_iter) ),*] }
            fn relations() -> Vec<bottle_orm::RelationInfo> { vec![#(#relations),*] }
            fn load_relations<'a>(
                relation_name: &'a str, models: &'a mut [Self], tx: &'a dyn bottle_orm::database::Connection,
                query_modifier: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
            ) -> futures::future::BoxFuture<'a, Result<(), sqlx::Error>> {
                Box::pin(async move {
                    if relation_name.is_empty() { return Ok(()); }
                    if relation_name.starts_with('(') && relation_name.ends_with(')') { return Self::load_relations(&relation_name[1..relation_name.len()-1], models, tx, query_modifier).await; }
                    let (mut parts, mut start, mut depth) = (Vec::new(), 0, 0);
                    for (i, c) in relation_name.char_indices() { match c { '(' => depth += 1, ')' => depth -= 1, '|' if depth == 0 => { parts.push(&relation_name[start..i]); start = i + 1; } _ => {} } }
                    parts.push(&relation_name[start..]);
                    for part in parts {
                        if part.is_empty() { continue; }
                        let (base_rel_raw, nested_rel) = if let Some(pos) = part.find('.') { (&part[..pos], &part[pos+1..]) } else { (part, "") };
                        let (base_rel, options) = if let Some(start_pos) = base_rel_raw.find('[') { if let Some(end_pos) = base_rel_raw.find(']') { (&base_rel_raw[..start_pos], &base_rel_raw[start_pos+1..end_pos]) } else { (base_rel_raw, "") } } else { (base_rel_raw, "") };
                        match base_rel { #(#load_relations_arms),* _ => return Err(sqlx::Error::Configuration(format!("Relation '{}' not found in model '{}'", base_rel, #table_name_str).into())), }
                    }
                    Ok(())
                })
            }
            fn to_map(&self) -> std::collections::HashMap<String, Option<String>> { let mut map = std::collections::HashMap::new(); #(#map_inserts)* map }
        }
        impl bottle_orm::AnyImpl for #struct_name { fn columns() -> Vec<bottle_orm::AnyInfo> { vec![#(#any_column_defs),*] } fn to_map(&self) -> std::collections::HashMap<String, Option<String>> { bottle_orm::Model::to_map(self) } }
        impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for #struct_name { fn from_row(row: &'r sqlx::any::AnyRow) -> Result<Self, sqlx::Error> { use sqlx::Row; #(#from_row_logic)* Ok(#struct_name { #(#field_names_construct),* }) } }
        impl bottle_orm::any_struct::FromAnyRow for #struct_name {
             fn from_any_row(row: &sqlx::any::AnyRow) -> Result<Self, sqlx::Error> { use sqlx::Row; #(#from_row_logic_clone)* Ok(#struct_name { #(#field_names_construct_clone),* }) }
             fn from_any_row_at(row: &sqlx::any::AnyRow, index: &mut usize) -> Result<Self, sqlx::Error> { use sqlx::Row; #(#from_row_logic_positional)* Ok(#struct_name { #(#field_names_construct_positional),* }) }
        }
    }
}
