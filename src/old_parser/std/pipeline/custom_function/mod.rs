use std::sync::Arc;
use crate::app::app_ctx::AppCtx;
use crate::core::item::Item;
use crate::core::items::logical::transform_with::TransformWithItem;
use crate::core::items::logical::valid::ValidItem;
use crate::core::items::logical::validate_with::ValidateWithItem;
use crate::parser::ast::argument::Argument;
use crate::prelude::Value;

pub(crate) fn custom_transform(args: &Vec<Argument>) -> Arc<dyn Item> {
    let lookup_table = AppCtx::get().unwrap().callbacks();
    if AppCtx::get().unwrap().ignore_callbacks() {
        return Arc::new(ValidItem::new());
    }
    let arg_value = args.get(0).unwrap().resolved.as_ref().unwrap().as_value().unwrap();
    match arg_value {
        Value::String(s) => {
            let name = s.as_str();
            let modifier = lookup_table.transform(name);
            if let Some(modifier) = modifier {
                modifier.clone()
            } else {
                panic!("Cannot find a transform named '{}'.", name)
            }
        }
        Value::Pipeline(p) => {
            Arc::new(TransformWithItem::new(p.clone()))
        }
        _ => panic!("Argument to `transform` should be string or pipeline.")
    }
}

pub(crate) fn custom_callback(args: &Vec<Argument>) -> Arc<dyn Item> {
    let lookup_table = AppCtx::get().unwrap().callbacks();
    if AppCtx::get().unwrap().ignore_callbacks() {
        Arc::new(ValidItem::new())
    } else {
        let name = args.get(0).unwrap().resolved.as_ref().unwrap().as_value().unwrap().as_str().unwrap();
        let modifier = lookup_table.callback(name);
        if let Some(modifier) = modifier {
            modifier.clone()
        } else {
            panic!("Cannot find a callback named '{}'.", name)
        }
    }

}

pub(crate) fn custom_validate(args: &Vec<Argument>) -> Arc<dyn Item> {
    let lookup_table = AppCtx::get().unwrap().callbacks();
    if AppCtx::get().unwrap().ignore_callbacks() {
        return Arc::new(ValidItem::new());
    }
    let arg_value = args.get(0).unwrap().resolved.as_ref().unwrap().as_value().unwrap();
    match arg_value {
        Value::String(s) => {
            let name = s.as_str();
            let modifier = lookup_table.validator(name);
            if let Some(modifier) = modifier {
                modifier.clone()
            } else {
                panic!("Cannot find a validate named '{}'.", name)
            }
        }
        Value::Pipeline(p) => {
            Arc::new(ValidateWithItem::new(p.clone()))
        }
        _ => panic!("Argument to `validate` should be string or pipeline.")
    }
}

pub(crate) fn custom_compare(args: &Vec<Argument>) -> Arc<dyn Item> {
    let lookup_table = AppCtx::get().unwrap().callbacks();
    if AppCtx::get().unwrap().ignore_callbacks() {
        return Arc::new(ValidItem::new());
    }
    let name = args.get(0).unwrap().resolved.as_ref().unwrap().as_value().unwrap().as_str().unwrap();
    let modifier = lookup_table.compare(name);
    if let Some(modifier) = modifier {
        modifier.clone()
    } else {
        panic!("Cannot find a compare named '{}'.", name)
    }
}