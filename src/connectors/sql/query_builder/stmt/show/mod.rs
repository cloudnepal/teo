use crate::connectors::sql::query_builder::stmt::show::index_from::SQLShowIndexFromStatement;
use crate::connectors::sql::query_builder::stmt::show::tables::SQLShowTablesStatement;

pub mod tables;
pub mod index_from;

pub struct SQLShowStatement { }

impl SQLShowStatement {
    pub fn tables(&self) -> SQLShowTablesStatement {
        SQLShowTablesStatement { like: None }
    }

    pub fn index_from(&self, table: impl Into<String>) -> SQLShowIndexFromStatement {
        SQLShowIndexFromStatement { table: table.into() }
    }
}