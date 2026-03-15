use axum::{Json, Router, extract::Query, response::Html, routing::get};
use chrono::{DateTime, Month};
use chrono_tz::Tz;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs::read_to_string, str::FromStr};

use crate::{
    context,
    group::Group,
    session::range::{Range, Span},
    task::Task,
    tasktree::{TaskTree, TotalTime},
    tree::Parent,
};

#[derive(Serialize, Deserialize)]
pub struct Node {
    name: String,
    value: i64,
    children: Vec<Node>,
}

pub fn root_node(task_tree: TaskTree, span: Span<DateTime<Tz>>, name: String) -> Node {
    Node {
        name,
        value: task_tree.time_delta(span).num_minutes(),
        children: task_tree
            .groups
            .iter()
            .map(|group| node_from_group(group, span))
            .filter(|node| node.value > 0)
            .collect(),
    }
}

// TODO: Add dynamic programming
fn node_from_group(group: &Group, span: Span<DateTime<Tz>>) -> Node {
    let task_nodes = Parent::<Task>::children(group)
        .iter()
        .map(|task| node_from_task(task, span));
    let group_nodes = Parent::<Group>::children(group)
        .iter()
        .map(|sub_group| node_from_group(sub_group, span));
    Node {
        name: group.text.clone(),
        value: group.time_delta(span).num_minutes(),
        children: group_nodes
            .chain(task_nodes)
            .filter(|node| node.value > 0)
            .collect(),
    }
}

// TODO: Add dynamic programming
fn node_from_task(task: &Task, span: Span<DateTime<Tz>>) -> Node {
    Node {
        name: task.text.clone(),
        value: task.time_delta(span).num_minutes(),
        children: Parent::<Task>::children(task)
            .iter()
            .map(|task| node_from_task(task, span))
            .filter(|node| node.value > 0)
            .collect(),
    }
}

// TODO: Add error handling
pub async fn serve() {
    // Try to aquire port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await;
    //
    open::that_in_background("http://127.0.0.1:3000");
    match listener {
        Ok(listener) => {
            let app = Router::<()>::new()
                .route("/", get(|| async { Html(include_str!("web/index.html")) }))
                .route("/data", get(get_data));
            let _result = axum::serve(listener, app).await;
        }
        Err(err) => {
            dbg!(err);
        }
    }
}

#[derive(Deserialize)]
// TODO: Add validated version, which implements display for root node name
struct RawParams {
    year: i32,
    month: Option<u8>,
    week: Option<u8>,
}

impl RawParams {
    // TODO: Add error handling
    fn range(&self) -> Option<Range> {
        match (self.month, self.week) {
            (Some(month), None) => Range::month(self.year, month.into()),
            (None, Some(week)) => Range::week(self.year, week.into()),
            _ => None,
        }
    }
}

impl Display for RawParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n", self.year)?;
        if let Some(num) = self.month
            && let Ok(month) = Month::try_from(num)
        {
            write!(f, "{}", month.name())?;
        }
        if let Some(week) = self.week {
            write!(f, "Week: {}", week)?;
        }
        Ok(())
    }
}

async fn get_data(Query(params): Query<RawParams>) -> Result<Json<Node>, (StatusCode, String)> {
    let todo = read_to_string(context::get().todo()).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to read file".into(),
        )
    })?;
    let todo_tree = TaskTree::from_str(&todo)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let done = read_to_string(context::get().done()).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to read file".into(),
        )
    })?;
    let done_tree = TaskTree::from_str(&done)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let span = params
        .range()
        .ok_or((
            StatusCode::BAD_REQUEST,
            "invalid or missing date range".into(),
        ))?
        .into_dt_span();
    let union = done_tree.union(todo_tree);
    Ok(Json(root_node(union, span, params.to_string())))
}
