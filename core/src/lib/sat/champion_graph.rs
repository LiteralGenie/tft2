use std::{ collections::{ HashMap, HashSet } };

use logicng::formulas::{ EncodedFormula, FormulaFactory };

use crate::lib::data::{ ChampionId, GameData };

use super::{
    build_subgraph_contraints,
    HashStringSet,
    SubgraphConstraints,
};

pub fn build_champion_constraints(
    subgraph_size: u8,
    slot_options: Vec<Vec<ChampionId>>
) -> (SubgraphConstraints, HashMap<i32, ChampionId>) {
    let data = GameData::new();

    // Map each champion id to arbitrary vertex index
    let id_to_index: HashMap<&ChampionId, i32> = HashMap::from_iter(
        data.champions
            .values()
            .enumerate()
            .map(|(idx, c)| (&c.name, idx as i32))
    );

    let index_to_id: HashMap<i32, ChampionId> = HashMap::from_iter(
        id_to_index.iter().map(|(k, v)| (v.clone(), (**k).clone()))
    );

    // Assign edges to champions that share traits
    let edges = HashSet::<(i32, i32)>::from_iter(
        data.champions
            .values()
            .flat_map(|c|
                c.traits.iter().flat_map(|t|
                    data.champions_by_trait
                        .get(t)
                        .unwrap()
                        .iter()
                        .map(|other| (&c.name, other))
                )
            )
            .map(|(id_1, id_2)| (
                id_to_index.get(id_1).unwrap().clone(),
                id_to_index.get(id_2).unwrap().clone(),
            ))
    );

    let n = data.champions.len() as i32;
    let mut constraints = build_subgraph_contraints(
        n,
        subgraph_size as i32,
        edges
    );

    let mut new_constraints = build_slot_constraints(
        slot_options,
        &constraints,
        id_to_index
    );

    // Merge subgraph and slot constraints
    constraints.num_constraints += new_constraints.len();

    new_constraints.push(constraints.formula);
    constraints.formula = constraints.factory.and(&new_constraints);

    (constraints, index_to_id)
}

struct ChampionSubset {
    champions: HashSet<ChampionId>,
    parent_sets: Vec<usize>,
}

impl ChampionSubset {
    fn new() -> Self {
        Self {
            champions: HashSet::new(),
            parent_sets: vec![],
        }
    }
}

fn build_slot_constraints(
    slot_options: Vec<Vec<ChampionId>>,
    subgraph_constraints: &SubgraphConstraints,
    id_to_index: HashMap<&ChampionId, i32>
) -> Vec<EncodedFormula> {
    let mut disjoint_subsets = Vec::<ChampionSubset>::new();

    let slot_options: Vec<HashSet<String>> = slot_options
        .into_iter()
        .map(|opts| HashSet::from_iter(opts.into_iter()))
        .collect();

    // Group each element by the parent sets it is common to
    for (idx, options) in slot_options.iter().enumerate() {
        let mut new_set = ChampionSubset::new();
        new_set.champions = HashSet::from_iter(options.clone());
        new_set.parent_sets.push(idx);

        merge_into_disjoint_list(new_set, &mut disjoint_subsets);
    }

    let mut constraints = Vec::<EncodedFormula>::new();
    let f = &subgraph_constraints.factory;

    // Solutions must contain at least one champion from each set of slot options
    for options in slot_options.iter() {
        constraints.push(
            f.or(
                &Vec::from_iter(
                    options
                        .iter()
                        .map(|id| format!("v{}", id_to_index[id]))
                        .map(|name| f.variable(name.as_str()))
                )
            )
        );
    }

    // A single champion cannot fill multiple slots
    //
    // For example, if we have two sets of slot options with common options
    //   A = { 1, 2 }
    //   B = { 1, 3 }
    //
    // Then the at-least-one constraints from above are
    //   (1|2) & (1|3)
    //
    // So 1,3 is a valid solution, but so is 1
    // To force the solution to have two elements, we add the additional constraint
    //   1 => 3|4     "If one is in the solution, so is 3 or 4"
    //
    // If we have three sets of slot options
    //   A = { 1, 2 }
    //   B = { 1, 3 }
    //   C = { 1, 4 }
    //
    // Then to prevent the length-1 solution, we need to add
    //   1      =>  2|3|4    "prevents 1"
    //
    // And to prevent length-2 solutions, we need to add
    //   1 & 2  =>  3|4      "prevents 1,2"
    //   1 & 3  =>  2|4      "prevents 1,3"
    //
    // Generalizing this to elements common to all sets in { S_0, S_1, S_2, ..., S_k },
    // means adding these constraints
    //   x_0        =>  x_other in { S_other - x_0 }
    //   x_0, x_1   =>  x_other in { S_other - x_0 - x_1 }
    //   ...
    //   x_0, ..., x_k => x_other in (S_other - x_0 - ... x_k)
    // for every combination of (x_0, ..., x_k) in the common set
    // where
    //   S_other = (S_0 union S_1 union S_2 ...)
    //   S_other - x_k = "all elements in S_other except for x_k"
    for set in disjoint_subsets {
        let n_parents = set.parent_sets.len();
        if n_parents <= 1 {
            continue;
        }

        let combinations = find_all_products(
            set.champions,
            n_parents
        );

        let mut union_of_parents = HashSet::new();
        for idx in set.parent_sets {
            let parent = slot_options[idx].clone();
            union_of_parents.extend(parent);
        }

        for lhs in combinations.iter() {
            let mut other_vars = union_of_parents.clone();
            for var in lhs.0.iter() {
                other_vars.remove(var);
            }

            let lhs_vars = champion_ids_to_vars(
                &lhs.0,
                &f,
                &id_to_index
            );

            let rhs_vars = champion_ids_to_vars(
                &other_vars,
                &f,
                &id_to_index
            );

            constraints.push(
                f.implication(f.and(&lhs_vars), f.or(&rhs_vars))
            );
        }
    }

    constraints
}

fn champion_ids_to_vars(
    ids: &HashSet<String>,
    factory: &FormulaFactory,
    id_to_index: &HashMap<&ChampionId, i32>
) -> Vec<EncodedFormula> {
    ids.iter()
        .map(|id_champion| format!("v{}", id_to_index[id_champion]))
        .map(|name| factory.variable(name.as_str()))
        .collect()
}

/**
 * Find all combinations of the options, up to some length and excluding repeats
 *
 * For example, given the arguments
 *   options = [ "a", "b", "c" ]
 *   length = 3
 *
 * The output will be
 *   {
 *      { "a" }
 *      { "b" }
 *      { "c" }
 *      { "a", "b" }
 *      { "a", "c" }
 *      { "a", "b", "c" }
 *   }
 */
fn find_all_products(
    options: HashSet<String>,
    length: usize
) -> HashSet<HashStringSet> {
    // Init with the options mapped to sets
    //   eg [ "a", "b", "c" ] -> { { "a" }, { "b" }, { "c" } }
    let mut iterations = vec![
        HashSet::from_iter(
            options
                .iter()
                .map(|s| HashStringSet::from_vec(&vec![s.clone()]))
        )
    ];

    // Take all the combinations of length N and find all combinations of length N+1
    for _ in 1..length {
        let mut to_add = HashSet::new();

        for set in iterations.last().unwrap().iter() {
            to_add.extend(product(set, &options));
        }

        iterations.push(to_add);
    }

    // Join the iterations into giant hash set
    let mut result = HashSet::new();
    for set in iterations {
        result.extend(set);
    }
    result
}

/**
 * Insert each option into new copies of source
 * eg for the arguments
 *   source  = { "a", "b" }
 *   options = [ "1", "2" ]
 * the output is
 *   {
 *      { "a", "1" },
 *      { "a", "2" },
 *      { "b", "1" },
 *      { "b", "2" }
 *   }
 */
fn product(
    source: &HashStringSet,
    options: &HashSet<String>
) -> HashSet<HashStringSet> {
    HashSet::from_iter(
        options.iter().map(|opt| {
            let mut s = source.clone();
            s.0.insert(opt.clone());
            s
        })
    )
}

/**
 * If new set looks something like this
 *    { 1, 6, 8 }
 *
 * and the list of existing (disjoint) sets looks like this
 *    [
 *       { 1, 3, 4 },
 *       { 2, 5, 7 },
 *    ]
 *
 * Any common elements like { 1 } are moved from both sets into its own set
 * The remaining elements are also given their own set.
 * ie the list is updated to look like this
 *    [
 *       { 3, 4 },
 *       { 2, 5, 7 },
 *       { 1 },
 *       { 6, 8 },
 *    ]
 *
 * And throughout this process we keep track of the original sets each of these subsets are common to
 * eg assuming our original sets looked something like this
 *    A: { 1, 2, 3, 4, 5, 7 }
 *    B: { 2, 5, 7 }
 *    C: { 1, 6, 8 }
 *
 * Then after the first merge, the disjoint list looks like
 *   [
 *      { 1, 3, 4 }   ->   parents: A
 *      { 2, 5, 7 }   ->   parents: A, B
 *   ]
 *
 * After the next merge, the list looks like this
 *   [
 *      { 3, 4 }      ->   parents: A
 *      { 2, 5, 7 }   ->   parents: A, B
 *       { 1 },        ->   parents: A, C
 *       { 6, 8 },     ->   parents: C
 *   ]
 */
fn merge_into_disjoint_list(
    mut new_set: ChampionSubset,
    disjoint_subsets: &mut Vec<ChampionSubset>
) {
    let mut to_append = Vec::<ChampionSubset>::new();

    for existing_subset in disjoint_subsets.iter_mut() {
        let mut common = ChampionSubset::new();

        // Elements from new_set that are common to existing_set
        // are common to (single) parent of new_set the sets that generated existing_set
        common.parent_sets.append(&mut new_set.parent_sets.clone());
        common.parent_sets.append(
            &mut existing_subset.parent_sets.clone()
        );

        // Find commons
        for champion in new_set.champions.iter() {
            let in_both =
                existing_subset.champions.contains(champion);

            if in_both {
                common.champions.insert(champion.clone());
            }
        }

        // Remove commons from parent sets
        for champion in common.champions.iter() {
            existing_subset.champions.remove(champion);
            new_set.champions.remove(champion);
        }

        if common.champions.len() > 0 {
            to_append.push(common);
        }
    }

    disjoint_subsets.append(&mut to_append);
}
