// Copyright 2021 Datafuse Labs.
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

use std::fs::File;
use std::io::Write;
use std::path::Path;

use common_datavalues::prelude::*;
use common_datavalues::type_coercion::numerical_arithmetic_coercion;
use common_datavalues::type_coercion::numerical_coercion;
use common_datavalues::type_coercion::numerical_unary_arithmetic_coercion;

pub fn codegen_arithmetic_type() {
    use DataValueBinaryOperator::*;
    use DataValueUnaryOperator::*;

    let dest = Path::new("common/datavalues/src/types");
    let path = dest.join("arithmetics_type.rs");

    let mut file = File::create(&path).expect("open");
    // Write the head.
    writeln!(
        file,
        "// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the \"License\");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an \"AS IS\" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// This code is generated by common/codegen. DO NOT EDIT.
use crate::IntegerType;
use crate::PrimitiveType;

pub trait ResultTypeOfBinary {{
    type AddMul: PrimitiveType;
    type Minus: PrimitiveType;
    type IntDiv: IntegerType;
    type Modulo: PrimitiveType;
    type LeastSuper: PrimitiveType;
}}

pub trait ResultTypeOfUnary {{
    type Negate: PrimitiveType;
}}"
    )
    .unwrap();

    let lhs = vec![
        UInt8Type::new_impl(),
        UInt16Type::new_impl(),
        UInt32Type::new_impl(),
        UInt64Type::new_impl(),
        Int8Type::new_impl(),
        Int16Type::new_impl(),
        Int32Type::new_impl(),
        Int64Type::new_impl(),
        Float32Type::new_impl(),
        Float64Type::new_impl(),
    ];
    let rhs = lhs.clone();
    for left in &lhs {
        for right in &rhs {
            let add_mul = numerical_arithmetic_coercion(&Plus, left, right).unwrap();
            let minus = numerical_arithmetic_coercion(&Minus, left, right).unwrap();
            let int_div = numerical_arithmetic_coercion(&IntDiv, left, right).unwrap();
            let modulo = numerical_arithmetic_coercion(&Modulo, left, right).unwrap();
            let least_super = numerical_coercion(left, right, true).unwrap();
            writeln!(
                file,
                "
impl ResultTypeOfBinary for ({}, {}) {{
    type AddMul = {};
    type Minus = {};
    type IntDiv = {};
    type Modulo = {};
    type LeastSuper = {};
}}",
                to_primitive_str(left.clone()),
                to_primitive_str(right.clone()),
                to_primitive_str(add_mul),
                to_primitive_str(minus),
                to_primitive_str(int_div),
                to_primitive_str(modulo),
                to_primitive_str(least_super),
            )
            .unwrap();
        }
    }

    for arg in &lhs {
        let negate = numerical_unary_arithmetic_coercion(&Negate, arg).unwrap();
        writeln!(
            file,
            "
impl ResultTypeOfUnary for {} {{
    type Negate = {};
}}",
            to_primitive_str(arg.clone()),
            to_primitive_str(negate),
        )
        .unwrap();
    }
    file.flush().unwrap();
}

fn to_primitive_str(dt: DataTypeImpl) -> &'static str {
    match dt.name().as_str() {
        "UInt8" => "u8",
        "UInt16" => "u16",
        "UInt32" => "u32",
        "UInt64" => "u64",
        "Int8" => "i8",
        "Int16" => "i16",
        "Int32" => "i32",
        "Int64" => "i64",
        "Float32" => "f32",
        "Float64" => "f64",
        _ => panic!("unsupported data type"),
    }
}
