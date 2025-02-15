// Copyright 2020 Datafuse Labs.
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
//

use common_datavalues::DataSchemaRef;
use common_dfs_api_vo::AppendResult;
use common_dfs_api_vo::BlockStream;
use common_dfs_api_vo::ReadAction;
use common_dfs_api_vo::ReadPlanResult;
use common_dfs_api_vo::TruncateTableResult;
use common_planners::ScanPlan;
use common_streams::SendableDataBlockStream;

#[async_trait::async_trait]
pub trait StorageApi: Send + Sync {
    async fn read_plan(
        &self,
        db_name: String,
        tbl_name: String,
        scan_plan: &ScanPlan,
    ) -> common_exception::Result<ReadPlanResult>;

    /// Get partition.
    async fn read_partition(
        &self,
        schema: DataSchemaRef,
        read_action: &ReadAction,
    ) -> common_exception::Result<SendableDataBlockStream>;

    async fn append_data(
        &self,
        db_name: String,
        tbl_name: String,
        scheme_ref: DataSchemaRef,
        mut block_stream: BlockStream,
    ) -> common_exception::Result<AppendResult>;

    async fn truncate(
        &self,
        db: String,
        table: String,
    ) -> common_exception::Result<TruncateTableResult>;
}
