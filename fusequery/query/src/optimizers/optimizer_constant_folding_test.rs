// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

#[cfg(test)]
mod tests {
    use common_exception::Result;
    use crate::optimizers::*;

    #[test]
    fn test_constant_folding_optimizer() -> Result<()> {
        #[allow(dead_code)]
        struct Test {
            name: &'static str,
            query: &'static str,
            expect: &'static str,
        }

        let tests: Vec<Test> = vec![
            Test {
                name: "Projection const recursion",
                query: "SELECT 1 + 2 + 3",
                expect: "\
                Projection: 6:UInt32\
                \n  Expression: 6:UInt32 (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection left non const recursion",
                query: "SELECT dummy + 1 + 2 + 3",
                expect: "\
                Projection: (((dummy + 1) + 2) + 3):UInt64\
                \n  Expression: (((dummy + 1) + 2) + 3):UInt64 (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection right non const recursion",
                query: "SELECT 1 + 2 + 3 + dummy",
                expect: "\
                Projection: (6 + dummy):UInt64\
                \n  Expression: (6 + dummy):UInt64 (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection arithmetic const recursion",
                query: "SELECT 1 + 2 + 3 / 3",
                expect: "\
                Projection: 4:Float64\
                \n  Expression: 4:Float64 (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection comparisons const recursion",
                query: "SELECT 1 + 2 + 3 > 3",
                expect: "\
                Projection: true:Boolean\
                \n  Expression: true:Boolean (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection cast const recursion",
                query: "SELECT CAST(1 AS bigint)",
                expect: "\
                Projection: 1:Int64\
                \n  Expression: 1:Int64 (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection hash const recursion",
                query: "SELECT sipHash('test_string')",
                expect: "\
                Projection: 17123704338732264132:UInt64\
                \n  Expression: 17123704338732264132:UInt64 (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection logics const recursion",
                query: "SELECT 1 = 1 AND 2 > 1",
                expect: "\
                Projection: true:Boolean\
                \n  Expression: true:Boolean (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection strings const recursion",
                query: "SELECT SUBSTRING('1234567890' FROM 3 FOR 3)",
                expect: "\
                Projection: 345:Utf8\
                \n  Expression: 345:Utf8 (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
            Test {
                name: "Projection to type name const recursion",
                query: "SELECT toTypeName('1234567890')",
                expect: "\
                Projection: Utf8:Utf8\
                \n  Expression: Utf8:Utf8 (Before Projection)\
                \n    ReadDataSource: scan partitions: [1], scan schema: [dummy:UInt8], statistics: [read_rows: 1, read_bytes: 1]",
            },
        ];

        for test in tests {
            let ctx = crate::tests::try_create_context()?;

            let plan = crate::tests::parse_query(test.query)?;
            let mut optimizer = ConstantFoldingOptimizer::create(ctx);
            let optimized = optimizer.optimize(&plan)?;
            let actual = format!("{:?}", optimized);
            assert_eq!(test.expect, actual, "{:#?}", test.name);
        }
        Ok(())
    }
}
