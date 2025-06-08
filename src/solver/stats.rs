use prettytable::{Cell, Row, Table};

use crate::solver::{
    constraint::Constraint,
    engine::{ConstraintId, PerConstraintStats, SearchStats},
    semantics::DomainSemantics,
};

pub fn render_stats_table<S: DomainSemantics>(
    stats: &SearchStats,
    constraints: &[Box<dyn Constraint<S>>],
) -> String {
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("Constraint Type"),
        Cell::new("ID"),
        Cell::new("Description"),
        Cell::new("Revise Calls"),
        Cell::new("Prunings"),
        Cell::new("Time / Call (µs)"),
        Cell::new("Total Time (ms)"),
    ]));

    let mut sorted_stats: Vec<(&ConstraintId, &PerConstraintStats)> =
        stats.constraint_stats.iter().collect();

    sorted_stats.sort_by_key(|a| a.1.time_spent_micros);

    for (constraint_id, constraint_stats) in sorted_stats {
        let descriptor = constraints[*constraint_id].descriptor();
        let avg_time = if constraint_stats.revisions > 0 {
            constraint_stats.time_spent_micros as f64 / constraint_stats.revisions as f64
        } else {
            0.0
        };

        table.add_row(Row::new(vec![
            Cell::new(&descriptor.name),
            Cell::new(&constraint_id.to_string()),
            Cell::new(&descriptor.description),
            Cell::new(&constraint_stats.revisions.to_string()),
            Cell::new(&constraint_stats.prunings.to_string()),
            Cell::new(&format!("{:.2}", avg_time)),
            Cell::new(&format!(
                "{:.2}",
                constraint_stats.time_spent_micros as f64 / 1000.0
            )),
        ]));
    }

    table.to_string()
}
