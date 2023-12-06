// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! `common_storage` will provide storage related types and functions.
//!
//! Databend Query will have three kinds of storage operators, visit
//! [RFC: Cache](https://databend.rs/doc/contributing/rfcs/cache) for
//! more detailed information.
//!
//! - data operator: All data will be persisted until users delete them.
//! - cache operator: Backends could have their GC or background auto eviction logic, which means cache services is non-persist.
//! - temporary operator: Backend will be configured with TTL and timely delete old data.
//!
//! Users can use different operator based on their own needs, for example:
//!
//! - Users table data must be accessed via data operator
//! - Table snapshots, segments cache must be stored accessed via cache operator.
//! - Intermediate data generated by query could be stored by temporary operator.

#![allow(clippy::uninlined_format_args)]

mod config;
pub use config::ShareTableConfig;
pub use config::StorageConfig;

mod operator;
pub use operator::init_operator;
pub use operator::DataOperator;

pub mod metrics;
pub use crate::metrics::StorageMetrics;
pub use crate::metrics::StorageMetricsLayer;

mod runtime_layer;

mod column_node;
pub use column_node::ColumnNode;
pub use column_node::ColumnNodes;

mod parquet2;
pub use parquet2::infer_schema_with_extension;
pub use parquet2::read_parquet_metas_in_parallel;
pub use parquet2::read_parquet_schema_async;

pub mod parquet_rs;
pub use parquet_rs::read_metadata_async;
pub use parquet_rs::read_parquet_schema_async_rs;

mod stage;
pub use stage::init_stage_operator;
pub use stage::StageFileInfo;
pub use stage::StageFileStatus;
pub use stage::StageFilesInfo;
pub use stage::STDIN_FD;

mod copy;
mod merge;
mod statistics;

pub use copy::CopyStatus;
pub use copy::FileParseError;
pub use copy::FileStatus;
pub use merge::MergeStatus;
pub use statistics::Datum;
pub use statistics::F64;
