//  Copyright 2021 Datafuse Labs.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//

use common_datavalues::DataSchemaRef;
use common_exception::Result;
use common_planners::TableOptions;

use crate::catalogs::Table;
use crate::common::StoreApiProvider;

pub trait TableEngine: Send + Sync {
    fn try_create(
        &self,
        db: String,
        name: String,
        schema: DataSchemaRef,
        options: TableOptions,
        store_provider: StoreApiProvider,
    ) -> Result<Box<dyn Table>>;
}

impl<T> TableEngine for T
where
    T: Fn(String, String, DataSchemaRef, TableOptions) -> Result<Box<dyn Table>>,
    T: Send + Sync,
{
    fn try_create(
        &self,
        db: String,
        name: String,
        schema: DataSchemaRef,
        options: TableOptions,
        _store_provider: StoreApiProvider,
    ) -> Result<Box<dyn Table>> {
        self(db, name, schema, options)
    }
}
