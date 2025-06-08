use std::{fmt, sync::Arc};

use im::HashMap;
use plico::solver::{
    constraint::Constraint,
    constraints::sum_of::SumOfConstraint,
    engine::{SolverEngine, VariableId},
    heuristics::{value::IdentityValueHeuristic, variable::SelectFirstHeuristic},
    semantics::DomainSemantics,
    solution::{Domain, RangeDomain, Solution},
    strategy::BacktrackingSearch,
    value::{StandardValue, ValueArithmetic, ValueRange},
};

// 1. Define problem-specific types
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BudgetValue(StandardValue);

impl From<StandardValue> for BudgetValue {
    fn from(sv: StandardValue) -> Self {
        BudgetValue(sv)
    }
}

impl ValueArithmetic for BudgetValue {
    fn add(&self, other: &Self) -> Self {
        BudgetValue(self.0.add(&other.0))
    }
    fn sub(&self, other: &Self) -> Self {
        BudgetValue(self.0.sub(&other.0))
    }
    fn abs(&self) -> Self {
        BudgetValue(self.0.abs())
    }
}

impl ValueRange for BudgetValue {
    fn successor(&self) -> Self {
        BudgetValue(self.0.successor())
    }
    fn distance(&self, other: &Self) -> u64 {
        self.0.distance(&other.0)
    }
}

// Custom debug formatting for currency
impl fmt::Display for BudgetValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let BudgetValue(StandardValue::Int(val)) = self {
            write!(f, "${:.2}", (*val as f64) / 100.0)
        } else {
            write!(f, "N/A")
        }
    }
}

#[derive(Debug, Clone)]
pub enum BudgetConstraint {
    SumOf(SumOfConstraint<BudgetSemantics>),
}

#[derive(Debug, Clone)]
pub struct BudgetSemantics;

impl DomainSemantics for BudgetSemantics {
    type Value = BudgetValue;
    type ConstraintDefinition = BudgetConstraint;
    type VariableMetadata = ();

    fn build_constraint(&self, def: &Self::ConstraintDefinition) -> Box<dyn Constraint<Self>> {
        match def {
            BudgetConstraint::SumOf(c) => Box::new(c.clone()),
        }
    }
}

fn main() {
    // 2. Define the problem instance
    let food: VariableId = 0;
    let utils: VariableId = 1;
    let rent: VariableId = 2;
    let entertainment: VariableId = 3;
    let candles: VariableId = 4;
    let total: VariableId = 5;

    let mut domains: HashMap<VariableId, Domain<BudgetValue>> = HashMap::new();
    domains.insert(
        food,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(20000)),
                BudgetValue(StandardValue::Int(80000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        utils,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(5000)),
                BudgetValue(StandardValue::Int(15000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        rent,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(120000)),
                BudgetValue(StandardValue::Int(120000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        entertainment,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(0)),
                BudgetValue(StandardValue::Int(50000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        candles,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(0)),
                BudgetValue(StandardValue::Int(2000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        total,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(0)),
                BudgetValue(StandardValue::Int(200000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );

    let semantics = Arc::new(BudgetSemantics);
    let initial_solution = Solution::new(domains, HashMap::new(), semantics.clone());

    let spending_vars = vec![food, utils, rent, entertainment, candles];
    let constraints = [BudgetConstraint::SumOf(SumOfConstraint::new(
        spending_vars,
        total,
    ))];
    let built = constraints
        .iter()
        .map(|c| semantics.build_constraint(c))
        .collect::<Vec<_>>();

    // 3. Run arc-consistency to prune domains
    println!("--- Initial Domains ---");
    print_domains(&initial_solution);

    let solver = SolverEngine::new(Box::new(BacktrackingSearch::new(
        Box::new(SelectFirstHeuristic),
        Box::new(IdentityValueHeuristic),
    )));
    // We only run arc_consistency, not the full search, to see the pruning.
    let mut stats = plico::solver::engine::SearchStats::default();
    let arc_consistent_solution = solver
        .arc_consistency(&built, initial_solution, &mut stats)
        .unwrap()
        .unwrap();

    println!("\n--- Pruned Domains (after arc-consistency) ---");
    print_domains(&arc_consistent_solution);
    println!("\n--- Stats ---");
    println!("{:#?}", stats);
}

fn print_domains<S: DomainSemantics<Value = BudgetValue>>(solution: &Solution<S>) {
    let names: HashMap<VariableId, &str> = [
        (0, "Food"),
        (1, "Utilities"),
        (2, "Rent"),
        (3, "Entertainment"),
        (4, "Candles"),
        (5, "Total"),
    ]
    .iter()
    .cloned()
    .collect();

    for (id, name) in &names {
        let domain = solution.domains.get(id).unwrap();
        let min_val = domain.get_min_value().unwrap();
        let max_val = domain.get_max_value().unwrap();
        println!("{:<15}: {} -> {}", name, min_val, max_val);
    }
}

#[test]
fn test_budget_arc_consistency() {
    // 1. Define the problem instance
    let food: VariableId = 0;
    let utils: VariableId = 1;
    let rent: VariableId = 2;
    let entertainment: VariableId = 3;
    let candles: VariableId = 4;
    let total: VariableId = 5;

    let mut domains: HashMap<VariableId, Domain<BudgetValue>> = HashMap::new();
    domains.insert(
        food,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(20000)),
                BudgetValue(StandardValue::Int(80000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        utils,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(5000)),
                BudgetValue(StandardValue::Int(15000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        rent,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(120000)),
                BudgetValue(StandardValue::Int(120000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        entertainment,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(0)),
                BudgetValue(StandardValue::Int(50000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        candles,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(0)),
                BudgetValue(StandardValue::Int(2000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );
    domains.insert(
        total,
        Box::new(
            RangeDomain::new(
                BudgetValue(StandardValue::Int(0)),
                BudgetValue(StandardValue::Int(200000)),
            )
            .unwrap(),
        ) as Domain<BudgetValue>,
    );

    let semantics = Arc::new(BudgetSemantics);
    let initial_solution = Solution::new(domains, HashMap::new(), semantics.clone());

    let spending_vars = vec![food, utils, rent, entertainment, candles];
    let constraints = [BudgetConstraint::SumOf(SumOfConstraint::new(
        spending_vars,
        total,
    ))];
    let built = constraints
        .iter()
        .map(|c| semantics.build_constraint(c))
        .collect::<Vec<_>>();

    // 2. Run arc-consistency to prune domains
    let solver = SolverEngine::new(Box::new(BacktrackingSearch::new(
        Box::new(SelectFirstHeuristic),
        Box::new(IdentityValueHeuristic),
    )));
    let mut stats = plico::solver::engine::SearchStats::default();
    let solution = solver
        .arc_consistency(&built, initial_solution, &mut stats)
        .unwrap()
        .unwrap();

    // 3. Assert final domains are correctly pruned
    let get_bounds = |sol: &Solution<BudgetSemantics>, id: VariableId| {
        let domain = sol.domains.get(&id).unwrap();
        (
            domain.get_min_value().unwrap(),
            domain.get_max_value().unwrap(),
        )
    };

    let (food_min, food_max) = get_bounds(&solution, food);
    let (utils_min, utils_max) = get_bounds(&solution, utils);
    let (rent_min, rent_max) = get_bounds(&solution, rent);
    let (entertainment_min, entertainment_max) = get_bounds(&solution, entertainment);
    let (candles_min, candles_max) = get_bounds(&solution, candles);
    let (total_min, total_max) = get_bounds(&solution, total);

    assert_eq!(
        food_min,
        BudgetValue(StandardValue::Int(20000)),
        "Food domain min should be 20000"
    );
    assert_eq!(
        food_max,
        BudgetValue(StandardValue::Int(75000)),
        "Food domain max should be 75000"
    );
    assert_eq!(
        utils_min,
        BudgetValue(StandardValue::Int(5000)),
        "Utilities domain min should be 5000"
    );
    assert_eq!(
        utils_max,
        BudgetValue(StandardValue::Int(15000)),
        "Utilities domain max should be 15000"
    );
    assert_eq!(
        rent_min,
        BudgetValue(StandardValue::Int(120000)),
        "Rent domain min should be 120000"
    );
    assert_eq!(
        rent_max,
        BudgetValue(StandardValue::Int(120000)),
        "Rent domain max should be 120000"
    );
    assert_eq!(
        entertainment_min,
        BudgetValue(StandardValue::Int(0)),
        "Entertainment domain min should be 0"
    );
    assert_eq!(
        entertainment_max,
        BudgetValue(StandardValue::Int(50000)),
        "Entertainment domain max should be 50000"
    );
    assert_eq!(
        candles_min,
        BudgetValue(StandardValue::Int(0)),
        "Candles domain min should be 0"
    );
    assert_eq!(
        candles_max,
        BudgetValue(StandardValue::Int(2000)),
        "Candles domain max should be 2000"
    );
    assert_eq!(
        total_min,
        BudgetValue(StandardValue::Int(145000)),
        "Total domain min should be 145000"
    );
    assert_eq!(
        total_max,
        BudgetValue(StandardValue::Int(200000)),
        "Total domain max should be 200000"
    );
}
