use async_trait::async_trait;
use crate::core::pipeline::modifier::Modifier;
use crate::core::pipeline::Pipeline;
use crate::core::pipeline::context::Context;
use crate::core::pipeline::context::stage::Stage::{ConditionFalse, ConditionTrue};
use crate::prelude::Value;

#[derive(Debug, Clone)]
pub struct FilterModifier {
    pipeline: Pipeline
}

impl FilterModifier {
    pub fn new(pipeline: Pipeline) -> Self {
        return FilterModifier {
            pipeline
        };
    }
}

#[async_trait]
impl Modifier for FilterModifier {

    fn name(&self) -> &'static str {
        "filter"
    }

    async fn call<'a>(&self, ctx: Context<'a>) -> Context<'a> {
        let mut retval = Vec::new();
        for (i, val) in ctx.value.as_vec().unwrap().iter().enumerate() {
            let item_ctx = ctx.alter_value(val.clone()).alter_key_path(&ctx.key_path + i);
            if self.pipeline.process(item_ctx.clone()).await.is_valid() {
                retval.push(item_ctx.value.clone());
            }
        }
        ctx.alter_value(Value::Vec(retval))
    }
}