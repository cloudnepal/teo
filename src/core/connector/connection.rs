use std::sync::Arc;
use async_trait::async_trait;
use crate::core::action::Action;
use crate::core::connector::session::SaveSession;
use crate::core::initiator::Initiator;
use crate::core::graph::Graph;
use crate::core::model::model::Model;
use crate::core::object::Object;
use crate::core::result::Result;
use crate::prelude::Value;

#[async_trait]
pub trait Connection: Send + Sync {

    // Migration

    async fn migrate(&self, models: Vec<&Model>, reset_database: bool) -> Result<()>;

    // Purge

    async fn purge(&self, graph: &Graph) -> Result<()>;

    // Raw query

    async fn query_raw(&self, query: &Value) -> Result<Value>;

    // Object manipulation

    async fn save_object(&self, object: &Object, session: Arc<dyn SaveSession>) -> Result<()>;

    async fn delete_object(&self, object: &Object, session: Arc<dyn SaveSession>) -> Result<()>;

    async fn find_unique<'a>(self: Arc<Self>, graph: &'static Graph, model: &'static Model, finder: &'a Value, mutation_mode: bool, action: Action, action_source: Initiator) -> Result<Option<Object>>;

    async fn find_many<'a>(self: Arc<Self>, graph: &'static Graph, model: &'static Model, finder: &'a Value, mutation_mode: bool, action: Action, action_source: Initiator) -> Result<Vec<Object>>;

    async fn count(&self, graph: &Graph, model: &Model, finder: &Value) -> Result<usize>;

    async fn aggregate(&self, graph: &Graph, model: &Model, finder: &Value) -> Result<Value>;

    async fn group_by(&self, graph: &Graph, model: &Model, finder: &Value) -> Result<Value>;

    // Save session

    fn new_save_session(&self) -> Arc<dyn SaveSession>;
}