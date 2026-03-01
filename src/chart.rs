#![allow(dead_code)]
use chrono::TimeDelta;
use eframe::{
    App, Frame, NativeOptions,
    egui::{CentralPanel, Context, Response, Ui},
    run_native,
};
use egui_plot::{Bar, BarChart, Legend, Plot};
use std::collections::HashMap;

struct Chart {
    bars: HashMap<String, Vec<TimeDelta>>,
}

impl App for Chart {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            show_plot(ui, &self.bars);
        });
    }
}

fn show(chart: Chart) -> eframe::Result {
    run_native(
        "My App",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(chart))),
    )
}

pub fn show_plot(ui: &mut Ui, bars: &HashMap<String, Vec<TimeDelta>>) -> Response {
    let mut sorted_keys: Vec<_> = bars.keys().collect();
    sorted_keys.sort();

    let mut charts: Vec<BarChart> = Vec::new();

    for (index, key) in sorted_keys.iter().enumerate() {
        let values = &bars[*key];
        let bar_data: Vec<Bar> = values
            .iter()
            .enumerate()
            .map(|(i, &duration)| {
                let hours = duration.num_hours() as f64;
                Bar::new(i as f64 + 0.5, hours)
            })
            .collect();

        let mut chart = BarChart::new(&format!("chart_{}", index), bar_data)
            .width(0.6)
            .name(key.as_str());

        if !charts.is_empty() {
            let chart_refs: Vec<&BarChart> = charts.iter().collect();
            chart = chart.stack_on(&chart_refs);
        }

        charts.push(chart);
    }

    Plot::new("Task Time Chart")
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            for chart in charts {
                plot_ui.bar_chart(chart);
            }
        })
        .response
}

// pub use demo::run_chart;

mod demo {
    use super::*;
    use chrono::{NaiveDate, TimeDelta};

    use crate::{
        session::{Span, first_time},
        tasktree::{TaskTree, TotalTime},
    };
    use std::{collections::HashMap, str::FromStr};

    pub fn run_chart() {
        let md = std::fs::read_to_string("/home/user/sync/notes/plan/todo.md").unwrap();
        let tasktree = TaskTree::from_str(&md).unwrap();
        let start = NaiveDate::from_ymd_opt(2026, 02, 1).unwrap();
        let days: Vec<NaiveDate> = start.iter_days().take(20).collect();
        let mut bars = HashMap::<String, Vec<TimeDelta>>::new();
        days.windows(2).for_each(|window| {
            let start = first_time(window.first().unwrap());
            let end = first_time(window.last().unwrap());
            let span = Span::new(start, end);
            tasktree.groups.iter().for_each(|group_item| {
                let key = group_item.text.clone();
                let value = group_item.time_delta(span);
                // if value > TimeDelta::zero() {
                bars.entry(key).or_insert_with(Vec::new).push(value);
                // }
            })
        });
        show(Chart { bars }).unwrap();
    }
}
