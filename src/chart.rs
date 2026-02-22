use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, TimeDelta};
use chrono_tz::Tz;
use eframe::{
    App, Frame, NativeOptions,
    egui::{CentralPanel, Context, Response, Ui},
    run_native,
};
use egui_plot::{Bar, BarChart, Legend, Plot};

use crate::tasktree::TaskTree;

struct Chart {
    bars: HashMap<String, Vec<TimeDelta>>,
}

impl App for Chart {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            show_plot(ui);
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

pub fn show_plot(ui: &mut Ui) -> Response {
    let chart1 = BarChart::new(
        "chart1",
        vec![
            Bar::new(0.5, 1.0).name("Day 1"),
            Bar::new(1.5, 3.0).name("Day 2"),
            Bar::new(2.5, 1.0).name("Day 3"),
            Bar::new(3.5, 2.0).name("Day 4"),
            Bar::new(4.5, 4.0).name("Day 5"),
        ],
    )
    .width(0.7)
    .name("Set 1");

    let chart2 = BarChart::new(
        "chart2",
        vec![
            Bar::new(0.5, 1.0),
            Bar::new(1.5, 1.5),
            Bar::new(2.5, 0.1),
            Bar::new(3.5, 0.7),
            Bar::new(4.5, 0.8),
        ],
    )
    .width(0.7)
    .name("Set 2")
    .stack_on(&[&chart1]);

    let chart3 = BarChart::new(
        "chart3",
        vec![
            Bar::new(0.5, 0.5),
            Bar::new(1.5, 1.0),
            Bar::new(2.5, 0.5),
            Bar::new(3.5, 1.0),
            Bar::new(4.5, 0.3),
        ],
    )
    .width(0.7)
    .name("Set 3")
    .stack_on(&[&chart1, &chart2]);

    let chart4 = BarChart::new(
        "chart4",
        vec![
            Bar::new(0.5, 0.5),
            Bar::new(1.5, 1.0),
            Bar::new(2.5, 0.5),
            Bar::new(3.5, 0.5),
            Bar::new(4.5, 0.5),
        ],
    )
    .width(0.7)
    .name("Set 4")
    .stack_on(&[&chart1, &chart2, &chart3]);

    Plot::new("Stacked Bar Chart Demo")
        .legend(Legend::default())
        .data_aspect(1.0)
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(chart1);
            plot_ui.bar_chart(chart2);
            plot_ui.bar_chart(chart3);
            plot_ui.bar_chart(chart4)
        })
        .response
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeDelta};

    use crate::{
        session::{Span, first_time},
        tasktree::{TaskTree, TotalTime},
    };
    use std::{collections::HashMap, str::FromStr};

    #[test]
    fn run_chart() {
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
                if value > TimeDelta::zero() {
                    bars.entry(key).or_insert_with(Vec::new).push(value);
                }
            })
        });
        show(Chart { bars }).unwrap();
    }
}
