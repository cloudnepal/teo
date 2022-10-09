use std::collections::{HashMap, HashSet};
use actix_web::http::header::Accept;
use bson::{Bson, DateTime as BsonDateTime, doc, Document, oid::ObjectId, Regex as BsonRegex};
use chrono::{Date, NaiveDate, Utc, DateTime};
use key_path::{KeyPath, path};
use crate::connectors::shared::has_negative_take::has_negative_take;
use crate::connectors::shared::map_has_i_mode::map_has_i_mode;
use crate::connectors::shared::query_pipeline_type::QueryPipelineType;
use crate::connectors::shared::user_json_args::user_json_args;
use crate::core::field::r#type::FieldType;
use crate::core::graph::Graph;
use crate::core::model::{Model};
use crate::core::tson::Value;
use crate::core::error::ActionError;
use crate::core::input::Input;
use crate::core::model::index::ModelIndexType;
use crate::tson;

fn insert_group_set_unset_for_aggregate(model: &Model, group: &mut Document, set: &mut Document, unset: &mut Vec<String>, k: &str, g: &str, having_mode: bool) {
    let prefix = if having_mode { "_having" } else { "" };
    let dbk = if k == "_all" { "_all" } else {model.field(k).unwrap().column_name() };
    if g == "count" {
        if k == "_all" {
            group.insert(format!("{prefix}_count__all"), doc!{"$count": {}});
        } else {
            group.insert(format!("{prefix}_count_{dbk}"), doc!{
                "$sum": {
                    "$cond": [{"$ifNull": [format!("${dbk}"), false]}, 1, 0]
                }
            });
        }
    } else {
        group.insert(format!("{prefix}_{g}_{dbk}"), doc!{format!("${g}"): format!("${dbk}")});
        if g == "sum" {
            group.insert(format!("{prefix}_{g}_count_{dbk}"), doc!{format!("$sum"): {
                "$cond": [
                    {"$ifNull": [format!("${dbk}"), false]},
                    1,
                    0
                ]
            }});
        }
    }
    if g == "sum" {
        set.insert(format!("{prefix}_{g}.{k}"), doc!{
            "$cond": {
                "if": {
                    "$eq": [format!("${prefix}_{g}_count_{dbk}"), 0]
                },
                "then": null,
                "else": format!("${prefix}_{g}_{dbk}")
            }
        });
        unset.push(format!("{prefix}_{g}_{dbk}"));
        unset.push(format!("{prefix}_{g}_count_{dbk}"));
    } else {
        set.insert(format!("{prefix}_{g}.{k}"), format!("${prefix}_{g}_{dbk}"));
        unset.push(format!("{prefix}_{g}_{dbk}"));
    }
}

fn build_query_pipeline(
    model: &Model,
    graph: &Graph,
    _type: QueryPipelineType,
    mutation_mode: bool,
    r#where: Option<&Value>,
    order_by: Option<&Value>,
    cursor: Option<&Value>,
    take: Option<i32>,
    skip: Option<i32>,
    page_size: Option<i32>,
    page_number: Option<i32>,
    include: Option<&Value>,
    distinct: Option<&Value>,
    select: Option<&Value>,
    aggregates: Option<&Value>,
    by: Option<&Value>,
    having: Option<&Value>,
    path: &KeyPath,
) -> Result<Vec<Document>, ActionError> {
    // cursor tweaks things so that we validate cursor first
    let cursor_additional_where = None;

    // $build the pipeline
    let mut retval: Vec<Document> = vec![];


    // group by contains it's own aggregates
    let empty_aggregates = tson!({});
    let the_aggregates = if by.is_some() {
        if aggregates.is_none() {
            Some(&empty_aggregates)
        } else {
            aggregates
        }
    } else {
        aggregates
    };
    // $aggregates at last
    if let Some(aggregates) = the_aggregates {
        let mut group = if let Some(by) = by {
            let mut id_for_group_by = doc!{};
            for key in by.as_vec().unwrap() {
                let k = key.as_str().unwrap();
                let dbk = model.field(k).unwrap().column_name();
                id_for_group_by.insert(dbk, doc!{
                    "$cond": [{"$ifNull": [format!("${dbk}"), false]}, format!("${dbk}"), null]
                });
            }
            doc!{"_id": id_for_group_by}
        } else {
            doc!{"_id": Bson::Null}
        };
        let mut set = doc!{};
        let mut unset: Vec<String> = vec![];
        if let Some(by) = by {
            for key in by.as_vec().unwrap() {
                let k = key.as_str().unwrap();
                let dbk = model.field(k).unwrap().column_name();
                set.insert(k, format!("$_id.{dbk}"));
            }
        }
        if let Some(having) = having {
            for (k, o) in having.as_hashmap().unwrap() {
                let _dbk = model.field(k).unwrap().column_name();
                for (g, _matcher) in o.as_hashmap().unwrap() {
                    let g = g.strip_prefix("_").unwrap();
                    insert_group_set_unset_for_aggregate(model, &mut group, &mut set, &mut unset, k, g, true);
                }
            }
        }
        for (g, o) in aggregates.as_hashmap().unwrap() {
            let g = g.strip_prefix("_").unwrap();
            for (k, _t) in o.as_hashmap().unwrap() {
                insert_group_set_unset_for_aggregate(model, &mut group, &mut set, &mut unset, k, g, false);
            }
        }
        retval.push(doc!{"$group": group});
        retval.push(doc!{"$set": set});
        if !unset.is_empty() {
            retval.push(doc!{"$unset": unset});
        }
        // filter if there is a having
        if let Some(having) = having {
            let mut having_match = doc!{};
            let mut having_unset: Vec<String> = Vec::new();
            for (k, o) in having.as_hashmap().unwrap() {
                let dbk = model.field(k).unwrap().column_name();
                for (g, matcher) in o.as_hashmap().unwrap() {
                    let g = g.strip_prefix("_").unwrap();
                    let matcher_bson = parse_bson_where_entry(&FieldType::F64, matcher, graph, &(path + "having" + k + format!("_{g}")))?;
                    having_match.insert(format!("_having_{g}.{dbk}"), matcher_bson);
                    let having_group = format!("_having_{g}");
                    if !having_unset.contains(&having_group) {
                        having_unset.push(having_group);
                    }
                }
            }
            retval.push(doc!{"$match": having_match});
            retval.push(doc!{"$unset": having_unset});
        }
        let mut group_by_sort = doc!{};
        if let Some(by) = by {
            // we need to order these
            for key in by.as_vec().unwrap() {
                let k = key.as_str().unwrap();
                group_by_sort.insert(k, 1);
            }
        }
        if !group_by_sort.is_empty() {
            retval.push(doc!{"$sort": group_by_sort});
        }
    }
    Ok(retval)
}
