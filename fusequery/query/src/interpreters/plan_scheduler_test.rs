// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

use std::sync::Arc;

use common_datavalues::DataValue;
use common_exception::Result;
use common_flights::Address;
use common_management::cluster::ClusterExecutor;
use common_planners::*;
use common_runtime::tokio;

use crate::api::FlightAction;
use crate::interpreters::plan_scheduler::PlanScheduler;
use crate::sessions::FuseQueryContextRef;
use crate::tests::try_create_cluster_context;
use crate::tests::ClusterNode;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_scheduler_plan_without_stage() -> Result<()> {
    let context = create_env().await?;
    let scheduler = PlanScheduler::try_create(context.clone())?;
    let scheduled_tasks = scheduler.reschedule(&PlanNode::Empty(EmptyPlan::create()))?;

    assert!(scheduled_tasks.get_tasks()?.is_empty());
    assert_eq!(
        scheduled_tasks.get_local_task(),
        PlanNode::Empty(EmptyPlan::create())
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_scheduler_plan_with_one_convergent_stage() -> Result<()> {
    /*
     *  +------------------+
     *  |                  |
     *  |     EmptyPlan    +--------------------------+
     *  |                  |                          |
     *  +------------------+                          |
     *                                       +--------v---------+
     *                                       |                  |
     *                                       |    Remote Plan   |
     *                                       |                  |
     *  +------------------+                 +--------^---------+
     *  |                  |                          |
     *  |     EmptyPlan    +--------------------------+
     *  |                  |
     *  +------------------+
     */
    let context = create_env().await?;
    let scheduler = PlanScheduler::try_create(context.clone())?;
    let scheduled_tasks = scheduler.reschedule(&PlanNode::Stage(StagePlan {
        kind: StageKind::Convergent,
        scatters_expr: Expression::create_literal(DataValue::UInt64(Some(0))),
        input: Arc::new(PlanNode::Empty(EmptyPlan::cluster())),
    }))?;

    let mut remote_actions = vec![];
    for (node, remote_action) in scheduled_tasks.get_tasks()? {
        match remote_action {
            FlightAction::BroadcastAction(_) => assert!(false),
            FlightAction::PrepareShuffleAction(action) => remote_actions.push((node, action)),
        }
    }

    assert_eq!(remote_actions.len(), 2);
    assert_eq!(remote_actions[0].0.name, String::from("dummy_local"));
    assert_eq!(remote_actions[0].1.sinks, vec![String::from("dummy_local")]);
    assert_eq!(
        remote_actions[0].1.scatters_expression,
        Expression::create_literal(DataValue::UInt64(Some(0)))
    );
    assert_eq!(
        remote_actions[0].1.plan,
        PlanNode::Empty(EmptyPlan::cluster())
    );

    assert_eq!(remote_actions[1].0.name, String::from("dummy"));
    assert_eq!(remote_actions[1].1.sinks, vec![String::from("dummy_local")]);
    assert_eq!(
        remote_actions[1].1.scatters_expression,
        Expression::create_literal(DataValue::UInt64(Some(0)))
    );
    assert_eq!(
        remote_actions[1].1.plan,
        PlanNode::Empty(EmptyPlan::cluster())
    );

    match scheduled_tasks.get_local_task() {
        PlanNode::Remote(plan) => {
            assert_eq!(plan.stream_id, "dummy_local");
            assert_eq!(plan.fetch_nodes, ["dummy_local", "dummy"]);
        }
        _ => assert!(
            false,
            "test_scheduler_plan_with_one_convergent_stage must be have Remote plan!"
        ),
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_scheduler_plan_with_convergent_and_expansive_stage() -> Result<()> {
    /*
     *                  +-----------+       +-----------+
     *        +-------->|RemotePlan +------>|SelectPlan +-----------+
     *        |         +-----------+       +-----------+           |
     *        |                                                     |
     *        |                                                     v
     *   +----+------+                                        +-----------+        +-----------+
     *   | EmptyPlan |                                        |RemotePlan +------->|SelectPlan |
     *   +----+------+                                        +-----------+        +-----------+
     *        |                                                     ^
     *        |         +-----------+       +-----------+           |
     *        +-------->|RemotePlan +------>|SelectPlan +-----------+
     *                  +-----------+       +-----------+
     */
    let context = create_env().await?;
    let scheduler = PlanScheduler::try_create(context.clone())?;
    let scheduled_tasks = scheduler.reschedule(&PlanNode::Select(SelectPlan {
        input: Arc::new(PlanNode::Stage(StagePlan {
            kind: StageKind::Convergent,
            scatters_expr: Expression::create_literal(DataValue::UInt64(Some(0))),
            input: Arc::new(PlanNode::Select(SelectPlan {
                input: Arc::new(PlanNode::Stage(StagePlan {
                    kind: StageKind::Expansive,
                    scatters_expr: Expression::ScalarFunction {
                        op: String::from("blockNumber"),
                        args: vec![],
                    },
                    input: Arc::new(PlanNode::Empty(EmptyPlan::create())),
                })),
            })),
        })),
    }))?;

    let mut remote_actions = vec![];
    for (node, remote_action) in scheduled_tasks.get_tasks()? {
        match remote_action {
            FlightAction::BroadcastAction(_) => assert!(false),
            FlightAction::PrepareShuffleAction(action) => remote_actions.push((node, action)),
        }
    }
    assert_eq!(remote_actions.len(), 3);
    assert_eq!(remote_actions[0].0.name, String::from("dummy_local"));
    assert_eq!(remote_actions[0].1.sinks, vec![
        String::from("dummy_local"),
        String::from("dummy")
    ]);
    assert_eq!(
        remote_actions[0].1.scatters_expression,
        Expression::ScalarFunction {
            op: String::from("blockNumber"),
            args: vec![],
        }
    );
    assert_eq!(
        remote_actions[0].1.plan,
        PlanNode::Empty(EmptyPlan::create())
    );

    assert_eq!(remote_actions[1].0.name, String::from("dummy_local"));
    assert_eq!(remote_actions[1].1.sinks, vec![String::from("dummy_local")]);
    assert_eq!(
        remote_actions[1].1.scatters_expression,
        Expression::create_literal(DataValue::UInt64(Some(0)))
    );

    assert_eq!(remote_actions[2].0.name, String::from("dummy"));
    assert_eq!(remote_actions[2].1.sinks, vec![String::from("dummy_local")]);
    assert_eq!(
        remote_actions[2].1.scatters_expression,
        Expression::create_literal(DataValue::UInt64(Some(0)))
    );

    // Perform the same plan in different nodes
    match (
        &remote_actions[1].1.plan,
        &remote_actions[2].1.plan,
        &scheduled_tasks.get_local_task(),
    ) {
        (PlanNode::Select(left), PlanNode::Select(right), PlanNode::Select(finalize)) => {
            match (&*left.input, &*right.input, &*finalize.input) {
                (PlanNode::Remote(left), PlanNode::Remote(right), PlanNode::Remote(finalize)) => {
                    assert_eq!(right.stream_id, "dummy");
                    assert_eq!(left.stream_id, "dummy_local");
                    assert_eq!(left.fetch_nodes, ["dummy_local"]);
                    assert_eq!(right.fetch_nodes, ["dummy_local"]);

                    assert_eq!(finalize.stream_id, "dummy_local");
                    assert_eq!(finalize.fetch_nodes, ["dummy_local", "dummy"]);
                },
                _ => assert!(false, "test_scheduler_plan_with_convergent_and_expansive_stage must be have Remote plan!"),
            }
        }
        _ => assert!(
            false,
            "test_scheduler_plan_with_convergent_and_expansive_stage must be have Select plan!"
        ),
    };

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_scheduler_plan_with_convergent_and_normal_stage() -> Result<()> {
    /*
     *   +-----------+      +-----------+       +-----------+
     *   |EmptyStage +----->|RemotePlan +------>|SelectPlan +-----------+
     *   +-------+---+      +-----------+       +-----------+           |
     *           |               ^                                      |
     *           +------------+  |                                      v
     *                        |  |                                +-----------+      +-----------+
     *           +------------+--+                                |RemotePlan +----> |SelectPlan |
     *           |            |                                   +-----------+      +-----------+
     *           |            v                                         ^
     *   +-------+---+      +-----------+       +-----------+           |
     *   |EmptyStage +----->|RemotePlan +------>|SelectPlan +-----------+
     *   +-----------+      +-----------+       +-----------+
     */
    let context = create_env().await?;
    let plan_scheduler = PlanScheduler::try_create(context.clone())?;
    let scheduled_tasks = plan_scheduler.reschedule(&PlanNode::Select(SelectPlan {
        input: Arc::new(PlanNode::Stage(StagePlan {
            kind: StageKind::Convergent,
            scatters_expr: Expression::create_literal(DataValue::UInt64(Some(1))),
            input: Arc::new(PlanNode::Select(SelectPlan {
                input: Arc::new(PlanNode::Stage(StagePlan {
                    kind: StageKind::Normal,
                    scatters_expr: Expression::create_literal(DataValue::UInt64(Some(0))),
                    input: Arc::new(PlanNode::Empty(EmptyPlan::cluster())),
                })),
            })),
        })),
    }))?;

    let mut remote_actions = vec![];
    for (node, remote_action) in scheduled_tasks.get_tasks()? {
        match remote_action {
            FlightAction::BroadcastAction(_) => assert!(false),
            FlightAction::PrepareShuffleAction(action) => remote_actions.push((node, action)),
        }
    }

    assert_eq!(remote_actions.len(), 4);
    assert_eq!(remote_actions[0].0.name, String::from("dummy_local"));
    assert_eq!(remote_actions[0].1.sinks, vec![
        String::from("dummy_local"),
        String::from("dummy")
    ]);
    assert_eq!(
        remote_actions[0].1.scatters_expression,
        Expression::create_literal(DataValue::UInt64(Some(0)))
    );
    assert_eq!(
        remote_actions[0].1.plan,
        PlanNode::Empty(EmptyPlan::cluster())
    );

    assert_eq!(remote_actions[2].0.name, String::from("dummy"));
    assert_eq!(remote_actions[2].1.sinks, vec![
        String::from("dummy_local"),
        String::from("dummy")
    ]);
    assert_eq!(
        remote_actions[2].1.scatters_expression,
        Expression::create_literal(DataValue::UInt64(Some(0)))
    );
    assert_eq!(
        remote_actions[2].1.plan,
        PlanNode::Empty(EmptyPlan::cluster())
    );

    assert_eq!(remote_actions[1].0.name, String::from("dummy_local"));
    assert_eq!(remote_actions[1].1.sinks, vec![String::from("dummy_local")]);
    assert_eq!(
        remote_actions[1].1.scatters_expression,
        Expression::create_literal(DataValue::UInt64(Some(1)))
    );

    assert_eq!(remote_actions[3].0.name, String::from("dummy"));
    assert_eq!(remote_actions[3].1.sinks, vec![String::from("dummy_local")]);
    assert_eq!(
        remote_actions[3].1.scatters_expression,
        Expression::create_literal(DataValue::UInt64(Some(1)))
    );

    // Perform the same plan in different nodes
    match (
        &remote_actions[1].1.plan,
        &remote_actions[3].1.plan,
        &scheduled_tasks.get_local_task(),
    ) {
        (PlanNode::Select(left), PlanNode::Select(right), PlanNode::Select(finalize)) => {
            match (&*left.input, &*right.input, &*finalize.input) {
                (PlanNode::Remote(left), PlanNode::Remote(right), PlanNode::Remote(finalize)) => {
                    assert_eq!(right.stream_id, "dummy");
                    assert_eq!(left.stream_id, "dummy_local");
                    assert_eq!(left.fetch_nodes, ["dummy_local", "dummy"]);
                    assert_eq!(right.fetch_nodes, ["dummy_local", "dummy"]);

                    assert_eq!(finalize.stream_id, "dummy_local");
                    assert_eq!(finalize.fetch_nodes, ["dummy_local", "dummy"]);
                },
                _ => assert!(false, "test_scheduler_plan_with_convergent_and_expansive_stage must be have Remote plan!"),
            }
        }
        _ => assert!(
            false,
            "test_scheduler_plan_with_convergent_and_expansive_stage must be have Select plan!"
        ),
    };

    Ok(())
}

async fn create_env() -> Result<FuseQueryContextRef> {
    try_create_cluster_context(&vec![
        ClusterNode::create("dummy_local", 1, "localhost:9090"),
        ClusterNode::create("dummy", 1, "github.com:9090"),
    ])
}
